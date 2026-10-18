//! FIB-6's lattice, proved against a brute-force enumerator, `engine::support`
//! and its own specification (D-6400 to D-6408). Every fixture is synthetic.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::too_many_lines,
    clippy::cast_possible_truncation,
    clippy::type_complexity,
    clippy::too_many_arguments,
    clippy::items_after_statements,
    reason = "a test that cannot panic cannot fail; its fixtures are spelled out where they are used"
)]

#[path = "fib6/fixture.rs"]
mod fixture;

use std::collections::{BTreeMap, BTreeSet, HashMap};

use fixture::{Seeded, base_cell, decision, first_side, generation, generation_with_edges};
use runner::fib::lattice::{
    Child, Decision, FIXED_SLOTS, LatticeRefusal, Node, Partition, RowTable, Schedule, Survivors,
    Tally, Visitor, Wanted, reconcile, walk, walk_guided,
};
use runner::fib::score::{CardSink, Gross, Outcome, Rungs, Slot, profiles_of};
use vocab::fib_cube::{
    Coords, CubeRefusal, Generation, Layout, MixKey, OPEN, ROW_COUNT, RowId, RowKind, Search,
    contains, untyped_numbers,
};

/// Keeps every visited node: its key, its rows and its fixed rows.
#[derive(Debug, Default, PartialEq)]
struct Collect {
    nodes: Vec<(MixKey, Vec<u32>, usize, Option<Vec<u8>>)>,
}

impl Visitor for Collect {
    fn visit(&mut self, node: &Node<'_>) {
        let wanted = match node.wanted() {
            Wanted::Every => None,
            Wanted::Only(variants) => Some(variants.to_vec()),
        };
        self.nodes
            .push((*node.key(), node.rows().to_vec(), node.fixed_rows(), wanted));
    }

    fn fork(&self) -> Self {
        Self::default()
    }

    fn absorb(&mut self, later: Self) {
        self.nodes.extend(later.nodes);
    }
}

/// The free rows, in row order.
fn free_rows() -> Vec<RowId> {
    RowId::ALL
        .into_iter()
        .filter(|id| id.row().kind != RowKind::Partition)
        .collect()
}

/// One random oracle case.
struct Case {
    generation: Generation,
    table: RowTable,
    cells: Vec<Coords>,
    varying: Vec<(RowId, Vec<u8>)>,
    min_support: u64,
}

/// A random case: at most six free rows vary, each over at most four values,
/// at most `max_rows` rows over one to three months.
fn random_case(rng: &mut Seeded, search: Search, max_rows: u64) -> Case {
    let measured = 2 + rng.below(3) as u8;
    let groups = 2 + rng.below(3) as u8;
    let generation = generation(search, measured, groups, 1);
    let layout = *generation.layout();
    let mut free = free_rows();
    let vary = 1 + rng.below(6) as usize;
    let mut varying = Vec::new();
    for _ in 0..vary {
        let id = free.remove(rng.below(free.len() as u64) as usize);
        let values = if id == RowId::Side {
            vec![first_side(search), fixture::second_side(search)]
        } else {
            let radix = u64::from(layout.radix(id));
            let mut set = BTreeSet::new();
            let want = 1 + rng.below(4.min(radix));
            while (set.len() as u64) < want {
                set.insert(rng.below(radix) as u8);
            }
            set.into_iter().collect()
        };
        varying.push((id, values));
    }
    varying.sort_by_key(|(id, _)| id.index());
    let rows = 1 + rng.below(max_rows);
    let months = 1 + rng.below(3) as u32;
    let mut data: Vec<(u32, Vec<(Decision, Vec<u32>)>)> =
        (0..months).map(|m| (m, Vec::new())).collect();
    for i in 0..rows {
        let mut cell = base_cell(search);
        for (id, values) in &varying {
            cell[id.index()] = values[rng.zipf(values.len() as u64) as usize];
        }
        let month = rng.below(u64::from(months)) as u32;
        let day = 1_000 + month * 31 + rng.below(28) as u32;
        let second = 33_300 + rng.below(21_000) as u32;
        let slot = rng.below(40) as u32;
        data[month as usize]
            .1
            .push((decision(cell, slot, day, second, i), vec![i as u32]));
    }
    let table = fixture::table_of(&generation, fixture::partition(search, 1), months, data);
    let cells = (0..table.len() as u32)
        .map(|r| *table.row(r).unwrap().coords)
        .collect();
    Case {
        generation,
        table,
        cells,
        varying,
        min_support: 1 + rng.below(50),
    }
}

/// The raw count of `mix` over `cells`, by `vocab::fib_cube::contains`.
fn count(mix: &MixKey, cells: &HashMap<Coords, u64>) -> u64 {
    cells
        .iter()
        .filter(|(cell, _)| contains(mix, cell))
        .map(|(_, n)| *n)
        .sum()
}

/// `mix` with row `id` set to `value`.
fn with(mix: MixKey, id: RowId, value: u8) -> MixKey {
    let mut out = mix;
    out.0[id.index()] = value;
    out
}

/// Whether the specification's chain rule visits `mix`: the root reaches the
/// floor, and each fixing in row order keeps the floor, changes the count,
/// and (on the cumulative row) differs from the prefix one group narrower.
fn chain_visits(root: MixKey, mix: &MixKey, cells: &HashMap<Coords, u64>, min: u64) -> bool {
    let mut at = root;
    let mut parent = count(&at, cells);
    if parent < min {
        return false;
    }
    for id in RowId::ALL {
        let value = mix.0[id.index()];
        if value == OPEN || at.0[id.index()] == value {
            continue;
        }
        let child = with(at, id, value);
        let n = count(&child, cells);
        if n < min || n == parent {
            return false;
        }
        if id.row().kind == RowKind::Cumulative && value > 0 {
            let narrower = count(&with(at, id, value - 1), cells);
            if narrower == n {
                return false;
            }
        }
        at = child;
        parent = n;
    }
    true
}

/// Every mix over the varying rows (each open or one of its values) that the
/// chain rule visits, with its count: the brute-force enumerator.
fn brute_force(case: &Case) -> BTreeMap<MixKey, u64> {
    let mut cells: HashMap<Coords, u64> = HashMap::new();
    for cell in &case.cells {
        *cells.entry(*cell).or_default() += 1;
    }
    let root = case.table.root_key();
    let mut mixes = vec![root];
    for (id, values) in &case.varying {
        let mut next = Vec::new();
        for mix in &mixes {
            next.push(*mix);
            for value in values {
                next.push(with(*mix, *id, *value));
            }
        }
        mixes = next;
    }
    mixes
        .into_iter()
        .filter(|mix| chain_visits(root, mix, &cells, case.min_support))
        .map(|mix| (mix, count(&mix, &cells)))
        .collect()
}

/// The specification's classification, re-stated.
fn classify(count: u64, parent: u64, sibling: Option<u64>, min: u64) -> Child {
    if count < min {
        Child::Infrequent
    } else if count == parent {
        Child::EquivalentToParent
    } else if sibling == Some(count) {
        Child::EquivalentToSibling
    } else {
        Child::Frequent
    }
}

/// A naive walk: every child of every visited node classified by filtering,
/// recursing on the frequent ones. Returns the tally (row-touches aside) and
/// the visited keys in visit order.
fn naive(layout: &Layout, root: MixKey, cells: &[Coords], min: u64) -> (Tally, Vec<MixKey>) {
    fn node(
        layout: &Layout,
        free: &[RowId],
        key: MixKey,
        rows: &[Coords],
        next: usize,
        fixed: usize,
        min: u64,
        tally: &mut Tally,
        order: &mut Vec<MixKey>,
    ) {
        order.push(key);
        for (position, id) in free.iter().enumerate().skip(next) {
            let cumulative = id.row().kind == RowKind::Cumulative;
            let mut sibling = None;
            // One counting pass per free row; a categorical row scatters once
            // when any child is frequent; a prefix child is one scan.
            tally.row_touches += rows.len() as u64;
            let mut any = false;
            for value in 0..layout.radix(*id) {
                let child: Vec<Coords> = rows
                    .iter()
                    .filter(|c| {
                        if cumulative {
                            c[id.index()] <= value
                        } else {
                            c[id.index()] == value
                        }
                    })
                    .copied()
                    .collect();
                let n = child.len() as u64;
                let class = classify(n, rows.len() as u64, sibling, min);
                if cumulative {
                    sibling = Some(n);
                }
                let array = match class {
                    Child::Infrequent => &mut tally.infrequent,
                    Child::EquivalentToParent => &mut tally.equivalent_to_parent,
                    Child::EquivalentToSibling => &mut tally.equivalent_to_sibling,
                    Child::Frequent => &mut tally.frequent,
                };
                array[fixed + 1] += 1;
                if class == Child::Frequent {
                    any = true;
                    if cumulative {
                        tally.row_touches += rows.len() as u64;
                    }
                    node(
                        layout,
                        free,
                        with(key, *id, value),
                        &child,
                        position + 1,
                        fixed + 1,
                        min,
                        tally,
                        order,
                    );
                }
            }
            if any && !cumulative {
                tally.row_touches += rows.len() as u64;
            }
        }
    }
    let mut tally = Tally::zero();
    let mut order = Vec::new();
    if (cells.len() as u64) < min {
        tally.infrequent[0] += 1;
    } else {
        tally.frequent[0] += 1;
        node(
            layout,
            &free_rows(),
            root,
            cells,
            0,
            0,
            min,
            &mut tally,
            &mut order,
        );
    }
    (tally, order)
}

/// Walks a table sequentially into a [`Collect`].
fn collect(table: &RowTable, min: u64) -> (Collect, Tally) {
    let mut visitor = Collect::default();
    let walked = walk(table, min, Schedule::Sequential, &mut visitor).unwrap();
    (visitor, walked.tally)
}

/// THE ORACLE (FIB-6): on seeded random inputs — at most six varying rows of at
/// most four values, at most 2,000 rows, a floor in 1..=50 — the visited mixes
/// and their counts equal the brute-force enumerator's, and every count equals
/// `engine::support` over the cells' `to_mask` bits.
#[test]
fn the_visited_mixes_and_counts_equal_a_brute_force_enumerator_and_engine_support() {
    let mut rng = Seeded(0x0F1B_0006);
    let (mut mixes, mut deepest, mut prefixes) = (0, 0, 0);
    for case_number in 0..60 {
        let search = if case_number % 2 == 0 {
            Search::Long
        } else {
            Search::Short
        };
        let case = random_case(&mut rng, search, 2_000);
        let (visited, _) = collect(&case.table, case.min_support);
        let got: BTreeMap<MixKey, u64> = visited
            .nodes
            .iter()
            .map(|(key, rows, _, _)| (*key, rows.len() as u64))
            .collect();
        assert_eq!(got.len(), visited.nodes.len(), "no mix is visited twice");
        let want = brute_force(&case);
        assert_eq!(got, want, "case {case_number}");
        let layout = case.generation.layout();
        let bits: Vec<_> = case
            .cells
            .iter()
            .map(|cell| layout.to_mask(cell).unwrap())
            .collect();
        for (key, rows, fixed, _) in &visited.nodes {
            let mix = with(*key, RowId::Exit, 0);
            let mask = layout.mix_mask(&mix).unwrap();
            assert_eq!(engine::support(&bits, &mask), rows.len() as u64);
            deepest = deepest.max(*fixed);
            prefixes += usize::from(key.0[RowId::MoverRank.index()] != OPEN);
        }
        mixes += visited.nodes.len();
    }
    assert!(mixes > 1_000, "the oracle compared {mixes} mixes");
    assert!(
        deepest >= 4,
        "a mix fixing {deepest} rows was the most compared"
    );
    assert!(prefixes > 20, "{prefixes} cumulative-row prefixes compared");
}

/// The tally, by fixed rows in the mix, equals a naive walk's classification
/// of every child, and the visit order is the naive walk's preorder.
#[test]
fn the_tally_equals_a_naive_classification_of_every_child() {
    let mut rng = Seeded(0x7A11_0006);
    for _ in 0..25 {
        let case = random_case(&mut rng, Search::Long, 600);
        let (visited, tally) = collect(&case.table, case.min_support);
        let (want, order) = naive(
            case.generation.layout(),
            case.table.root_key(),
            &case.cells,
            case.min_support,
        );
        assert_eq!(tally.frequent, want.frequent);
        assert_eq!(tally.infrequent, want.infrequent);
        assert_eq!(tally.equivalent_to_parent, want.equivalent_to_parent);
        assert_eq!(tally.equivalent_to_sibling, want.equivalent_to_sibling);
        assert_eq!(
            tally.row_touches, want.row_touches,
            "every row-touch accounted for"
        );
        let keys: Vec<MixKey> = visited.nodes.iter().map(|(k, ..)| *k).collect();
        assert_eq!(keys, order);
        assert_eq!(tally.nodes(), keys.len() as u64);
        assert_eq!(tally.mixes(3), 3 * tally.nodes());
        assert!(tally.row_touches > 0);
    }
}

/// Every node's rows are ascending (canonical time order), and the cumulative
/// row's children are nested prefixes: a `group <= g` child holds exactly the
/// node's rows of groups up to `g` (FM-F055).
#[test]
fn prefix_children_rows_in_canonical_order_and_nested() {
    let mut rng = Seeded(0x9E57_0006);
    for _ in 0..20 {
        let mut case = random_case(&mut rng, Search::Long, 1_500);
        case.min_support = 1 + case.min_support / 10;
        let (visited, _) = collect(&case.table, case.min_support);
        for (key, rows, _, _) in &visited.nodes {
            assert!(rows.windows(2).all(|w| w[0] < w[1]), "rows ascending");
            let want: Vec<u32> = (0..case.cells.len() as u32)
                .filter(|r| contains(key, &case.cells[*r as usize]))
                .collect();
            assert_eq!(rows, &want, "{key:?}");
        }
        for r in 1..case.table.len() as u32 {
            let a = case.table.row(r - 1).unwrap();
            let b = case.table.row(r).unwrap();
            assert!((a.day, a.second, a.slot, a.id) < (b.day, b.second, b.slot, b.id));
        }
    }
}

/// Raising the floor never adds a mix.
#[test]
fn raising_min_support_never_adds_a_mix() {
    let mut rng = Seeded(0x5EED_0006);
    for _ in 0..10 {
        let case = random_case(&mut rng, Search::Short, 1_500);
        let mut previous: Option<BTreeSet<MixKey>> = None;
        for min in [1, 2, 3, 5, 8, 13, 21, 34, 55, 89] {
            let (visited, _) = collect(&case.table, min);
            let keys: BTreeSet<MixKey> = visited.nodes.iter().map(|(k, ..)| *k).collect();
            if let Some(lower) = &previous {
                assert!(keys.is_subset(lower), "floor {min} added a mix");
            }
            previous = Some(keys);
        }
    }
}

/// A gate-like visitor: the share of rows whose outcome index is even.
#[derive(Default)]
struct Gate {
    verdicts: Vec<(MixKey, bool)>,
}

impl Visitor for Gate {
    fn visit(&mut self, node: &Node<'_>) {
        let table = node.table();
        assert_eq!(node.support(), node.rows().len() as u64);
        assert!(node.support() >= 5, "every visited node reaches the floor");
        let wins = node
            .rows()
            .iter()
            .filter(|r| table.row(**r).unwrap().outcomes[0] % 2 == 0)
            .count();
        self.verdicts
            .push((*node.key(), wins * 100 >= node.rows().len() * 51));
    }

    fn fork(&self) -> Self {
        Self::default()
    }

    fn absorb(&mut self, later: Self) {
        self.verdicts.extend(later.verdicts);
    }
}

/// The lattice never reads a verdict: a child of a parent that fails a
/// gate-like test is still visited, and can pass it (operator 11:10 IST).
#[test]
fn a_child_whose_parent_fails_the_gate_is_still_visited_and_can_pass() {
    let generation = generation(Search::Long, 2, 2, 1);
    let mut data = Vec::new();
    for i in 0..40_u32 {
        let mut cell = base_cell(Search::Long);
        let mirror = i < 10;
        if mirror {
            cell[RowId::Side.index()] = 3;
        }
        let outcome = if mirror {
            2 * i
        } else {
            2 * i + u32::from(i % 4 != 0)
        };
        data.push((
            decision(cell, i, 2_000, 40_000 + i, u64::from(i)),
            vec![outcome],
        ));
    }
    let table = fixture::table_of(
        &generation,
        fixture::partition(Search::Long, 1),
        1,
        vec![(0, data)],
    );
    let mut gate = Gate::default();
    walk(&table, 5, Schedule::Sequential, &mut gate).unwrap();
    let root = table.root_key();
    let child = with(root, RowId::Side, 3);
    let verdict = |key: MixKey| {
        gate.verdicts
            .iter()
            .find(|(k, _)| *k == key)
            .map(|(_, v)| *v)
    };
    assert_eq!(verdict(root), Some(false), "the root fails");
    assert_eq!(
        verdict(child),
        Some(true),
        "its child is visited and passes"
    );
}

/// A child equal to its parent is skipped as `equivalent_to_parent`, counted
/// at its fixed rows, never visited; so is every all-one-cell child.
#[test]
fn equivalent_to_parent_is_skipped_and_counted() {
    let generation = generation(Search::Long, 3, 2, 1);
    let mut data = Vec::new();
    for i in 0..30_u32 {
        let mut cell = base_cell(Search::Long);
        cell[RowId::FibZone.index()] = 2;
        cell[RowId::Weekday.index()] = (i % 3) as u8;
        data.push((decision(cell, i, 3_000, 40_000 + i, u64::from(i)), vec![i]));
    }
    let table = fixture::table_of(
        &generation,
        fixture::partition(Search::Long, 1),
        1,
        vec![(0, data)],
    );
    let (visited, tally) = collect(&table, 1);
    let zone = with(table.root_key(), RowId::FibZone, 2);
    assert!(visited.nodes.iter().all(|(k, ..)| *k != zone));
    assert!(tally.equivalent_to_parent[1] >= 1);
    let keys: BTreeSet<MixKey> = visited.nodes.iter().map(|(k, ..)| *k).collect();
    assert_eq!(keys.len(), 4, "the root and its three weekday children");
    assert_eq!(tally.frequent[0], 1);
    assert_eq!(tally.frequent[1], 3);
}

/// A repeated inner rank edge leaves a group empty: its prefix equals the one
/// before it and is skipped as `equivalent_to_sibling` (FM-F103, FM-F005).
#[test]
fn equivalent_to_sibling_is_skipped_and_counted() {
    let generation = generation_with_edges(Search::Long, 2, vec![2, 2, 5], 1);
    let mut data = Vec::new();
    for i in 0..30_u32 {
        let mut cell = base_cell(Search::Long);
        cell[RowId::MoverRank.index()] = [0, 2, 3][(i % 3) as usize];
        data.push((decision(cell, i, 3_000, 40_000 + i, u64::from(i)), vec![i]));
    }
    let table = fixture::table_of(
        &generation,
        fixture::partition(Search::Long, 1),
        1,
        vec![(0, data)],
    );
    let (visited, tally) = collect(&table, 1);
    assert_eq!(tally.equivalent_to_sibling[1], 1, "group 1 equals group 0");
    assert!(tally.equivalent_to_parent[1] >= 1, "group 3 is everything");
    let groups: BTreeSet<u8> = visited
        .nodes
        .iter()
        .map(|(k, ..)| k.0[RowId::MoverRank.index()])
        .collect();
    assert_eq!(groups, BTreeSet::from([0, 2, OPEN]));
    let narrow = with(table.root_key(), RowId::MoverRank, 2);
    let rows = &visited.nodes.iter().find(|(k, ..)| *k == narrow).unwrap().1;
    assert_eq!(rows.len(), 20, "group <= 2 holds groups 0 and 2");
}

/// Empty input, one row, and every row in one cell.
#[test]
fn empty_input_one_row_and_every_row_in_one_cell() {
    let generation = generation(Search::Long, 2, 2, 1);
    let empty = fixture::table_of(&generation, fixture::partition(Search::Long, 1), 1, vec![]);
    let (visited, tally) = collect(&empty, 1);
    assert!(visited.nodes.is_empty());
    assert_eq!(tally.infrequent[0], 1);
    assert_eq!(tally.nodes(), 0);
    assert!(empty.is_empty());

    let one = fixture::table_of(
        &generation,
        fixture::partition(Search::Long, 1),
        1,
        vec![(
            0,
            vec![(decision(base_cell(Search::Long), 0, 5, 40_000, 1), vec![0])],
        )],
    );
    let (visited, tally) = collect(&one, 1);
    assert_eq!(
        visited.nodes.len(),
        1,
        "only the root: every child equals it or is empty"
    );
    assert_eq!(tally.frequent[0], 1);
    assert_eq!(tally.frequent[1..], [0; FIXED_SLOTS - 1]);
    let (none, _) = collect(&one, 2);
    assert!(none.nodes.is_empty());

    let same: Vec<_> = (0..50_u32)
        .map(|i| {
            (
                decision(base_cell(Search::Long), i, 5, 40_000, u64::from(i)),
                vec![i],
            )
        })
        .collect();
    let cell = fixture::table_of(
        &generation,
        fixture::partition(Search::Long, 1),
        1,
        vec![(0, same)],
    );
    let (visited, tally) = collect(&cell, 1);
    assert_eq!(visited.nodes.len(), 1);
    assert_eq!(visited.nodes[0].1.len(), 50);
    assert!(tally.equivalent_to_parent[1] > 0);
}

/// FM-F051: month blocks appended one at a time equal one full sort of every
/// decision by (day, second, slot, id).
#[test]
fn month_block_order_equals_full_sort() {
    let mut rng = Seeded(0x0B10_C006);
    let generation = generation(Search::Long, 2, 2, 2);
    for _ in 0..10 {
        let months = 1 + rng.below(6) as u32;
        let mut all = Vec::new();
        let mut data = Vec::new();
        let mut id = 0;
        for month in 0..months {
            let mut decisions = Vec::new();
            for _ in 0..rng.below(300) {
                let d = decision(
                    base_cell(Search::Long),
                    rng.below(30) as u32,
                    10_000 + month * 31 + rng.below(30) as u32,
                    33_300 + rng.below(50) as u32,
                    id,
                );
                id += 1;
                all.push(d);
                decisions.push((d, vec![d.id as u32 * 2, d.id as u32 * 2 + 1]));
            }
            data.push((month, decisions));
        }
        let table = fixture::table_of(
            &generation,
            fixture::partition(Search::Long, 2),
            months,
            data,
        );
        all.sort_by_key(|d| (d.day, d.second, d.slot, d.id));
        assert_eq!(table.len(), all.len());
        for (r, want) in all.iter().enumerate() {
            let got = table.row(r as u32).unwrap();
            assert_eq!(
                (got.day, got.second, got.slot, got.id),
                (want.day, want.second, want.slot, want.id)
            );
            assert_eq!(got.outcomes, &[want.id as u32 * 2, want.id as u32 * 2 + 1]);
        }
        assert!(table.row(all.len() as u32).is_none());
        assert_eq!(
            table.order(),
            (0..all.len() as u32).collect::<Vec<u32>>().as_slice()
        );
        assert_eq!(table.layout(), generation.layout());
        assert_eq!(table.is_empty(), all.is_empty());
    }
}

/// Every refusal of a table is by name, and a refused month leaves the table
/// as it was.
#[test]
fn a_table_refuses_by_name_and_a_refused_month_changes_nothing() {
    let generation = generation(Search::Long, 2, 2, 2);
    let layout = *generation.layout();
    let part = fixture::partition(Search::Long, 2);
    assert_eq!(
        RowTable::new(layout, part.clone(), 0),
        Err(LatticeRefusal::MonthsZero)
    );
    let bad_style = Partition {
        entry_style: 3,
        ..part.clone()
    };
    assert!(matches!(
        RowTable::new(layout, bad_style, 1),
        Err(LatticeRefusal::PartitionValueOutOfRange {
            row: "entry_style",
            value: 3,
            values: 3
        })
    ));
    let bad_width = Partition {
        timeframe: 1,
        ..part.clone()
    };
    assert!(matches!(
        RowTable::new(layout, bad_width, 1),
        Err(LatticeRefusal::PartitionValueOutOfRange {
            row: "timeframe",
            ..
        })
    ));
    for exits in [0..0, 1..3] {
        let bad = Partition {
            exits,
            ..part.clone()
        };
        assert!(matches!(
            RowTable::new(layout, bad, 1),
            Err(LatticeRefusal::ExitsInvalid { values: 2, .. })
        ));
    }
    let mut table = RowTable::new(layout, part, 3).unwrap();
    let ok = decision(base_cell(Search::Long), 1, 100, 40_000, 7);
    table.push_month(1, &[ok], &[0, 1]).unwrap();
    let before = table.clone();
    let refuse = |table: &mut RowTable, month, decisions: &[Decision], outcomes: &[u32]| {
        let refusal = table.push_month(month, decisions, outcomes).unwrap_err();
        assert_eq!(*table, before);
        refusal
    };
    assert_eq!(
        refuse(&mut table, 3, &[ok], &[0, 1]),
        LatticeRefusal::MonthOutOfRange {
            month: 3,
            months: 3
        }
    );
    assert_eq!(
        refuse(&mut table, 1, &[ok], &[0, 1]),
        LatticeRefusal::MonthNotAfterLast { month: 1, last: 1 }
    );
    assert_eq!(
        refuse(&mut table, 2, &[ok], &[0, 1]),
        LatticeRefusal::MonthDaysOverlap {
            month: 2,
            first_day: 100,
            previous_last_day: 100
        }
    );
    let later = Decision { day: 200, ..ok };
    assert_eq!(
        refuse(&mut table, 2, &[later], &[0]),
        LatticeRefusal::OutcomesLength {
            decisions: 1,
            variants: 2,
            outcomes: 1
        }
    );
    let mut cell = later.coords;
    cell[RowId::FibZone.index()] = 9;
    assert!(matches!(
        refuse(
            &mut table,
            2,
            &[Decision {
                coords: cell,
                ..later
            }],
            &[0, 1]
        ),
        LatticeRefusal::Cell(CubeRefusal::ValueOutOfRange {
            row: "fib_zone",
            ..
        })
    ));
    let mut short = later.coords;
    short[RowId::Side.index()] = 1;
    assert_eq!(
        refuse(
            &mut table,
            2,
            &[Decision {
                coords: short,
                ..later
            }],
            &[0, 1]
        ),
        LatticeRefusal::SideOutsideSearch {
            search: "long",
            side: 1
        }
    );
    for (id, value) in [
        (RowId::EntryStyle, 1),
        (RowId::Timeframe, 0),
        (RowId::Exit, 1),
    ] {
        let mut cell = later.coords;
        cell[id.index()] = value;
        if id == RowId::Timeframe {
            continue;
        }
        assert_eq!(
            refuse(
                &mut table,
                2,
                &[Decision {
                    coords: cell,
                    ..later
                }],
                &[0, 1]
            ),
            LatticeRefusal::PartitionMismatch {
                row: id.row().name,
                want: 0,
                have: value
            }
        );
    }
    assert_eq!(
        refuse(
            &mut table,
            2,
            &[Decision {
                second: 86_400,
                ..later
            }],
            &[0, 1]
        ),
        LatticeRefusal::SecondOutOfDay { second: 86_400 }
    );
    let a = Decision {
        slot: 2,
        id: 9,
        ..later
    };
    let b = Decision {
        slot: 2,
        id: 8,
        ..later
    };
    assert_eq!(
        refuse(&mut table, 2, &[a, b], &[0, 1, 2, 3]),
        LatticeRefusal::InputNotInSlotOrder { at: 1 }
    );
    assert_eq!(
        refuse(&mut table, 2, &[a, a], &[0, 1, 2, 3]),
        LatticeRefusal::InputNotInSlotOrder { at: 1 }
    );
    table.push_month(2, &[], &[]).unwrap();
    assert_eq!(table.len(), 1, "an empty month adds no row");
    assert!(!table.is_empty());
    assert_eq!(table.pool(), 2);
    assert_eq!(table.months(), 3);
    assert_eq!(table.variants(), 2);
}

/// A floor of zero is refused by name.
#[test]
fn a_floor_of_zero_is_refused() {
    let generation = generation(Search::Long, 2, 2, 1);
    let table = fixture::table_of(&generation, fixture::partition(Search::Long, 1), 1, vec![]);
    let mut visitor = Collect::default();
    assert_eq!(
        walk(&table, 0, Schedule::Sequential, &mut visitor).unwrap_err(),
        LatticeRefusal::MinSupportZero
    );
}

/// A random table of several exit variants, its decisions and outcomes.
fn shared_case(rng: &mut Seeded, exits: u8, rows: u32) -> (Generation, Vec<(Decision, Vec<u32>)>) {
    let generation = generation(Search::Long, 3, 3, u16::from(exits));
    let layout = *generation.layout();
    let mut data = Vec::new();
    for i in 0..rows {
        let mut cell = base_cell(Search::Long);
        for id in [
            RowId::Weekday,
            RowId::BaseKind,
            RowId::MoverRank,
            RowId::TriggerBody,
            RowId::StopSize,
        ] {
            cell[id.index()] = rng.zipf(u64::from(layout.radix(id))) as u8;
        }
        if rng.below(3) == 0 {
            cell[RowId::Side.index()] = 3;
        }
        let outcomes = (0..u32::from(exits))
            .map(|v| i * u32::from(exits) + v)
            .collect();
        data.push((
            decision(
                cell,
                rng.below(25) as u32,
                4_000 + rng.below(20) as u32,
                33_300 + rng.below(20_000) as u32,
                u64::from(i),
            ),
            outcomes,
        ));
    }
    (generation, data)
}

/// FM-F054: one traversal scoring every exit variant at every node equals the
/// per-exit partition runs — the same (mix, rows) for every variant — and its
/// node visits equal one partition's.
#[test]
fn shared_traversal_equals_per_exit_partitions() {
    let mut rng = Seeded(0x5A4E_0006);
    let exits = 3_u8;
    let (generation, data) = shared_case(&mut rng, exits, 1_200);
    let shared = fixture::table_of(
        &generation,
        fixture::partition(Search::Long, exits),
        1,
        vec![(0, data.clone())],
    );
    let (visited, tally) = collect(&shared, 4);
    let mut from_shared = BTreeSet::new();
    for (key, rows, ..) in &visited.nodes {
        for v in 0..exits {
            let node_mix = with(*key, RowId::Exit, v);
            from_shared.insert((node_mix, rows.clone()));
        }
    }
    let mut from_parts = BTreeSet::new();
    for v in 0..exits {
        let part = Partition {
            exits: v..v + 1,
            ..fixture::partition(Search::Long, exits)
        };
        let decisions: Vec<(Decision, Vec<u32>)> = data
            .iter()
            .map(|(d, o)| {
                let mut cell = d.coords;
                cell[RowId::Exit.index()] = v;
                (Decision { coords: cell, ..*d }, vec![o[v as usize]])
            })
            .collect();
        let table = fixture::table_of(&generation, part, 1, vec![(0, decisions)]);
        let (one, one_tally) = collect(&table, 4);
        assert_eq!(one_tally, tally, "a partition's tally is the shared one");
        for (key, rows, ..) in &one.nodes {
            from_parts.insert((with(*key, RowId::Exit, v), rows.clone()));
        }
    }
    assert_eq!(from_shared, from_parts);
    assert_eq!(tally.mixes(usize::from(exits)), from_parts.len() as u64);
}

/// A node's mix names only the table's own exit variants.
#[test]
fn a_node_mixes_only_the_tables_exit_variants() {
    let generation = generation(Search::Long, 2, 2, 4);
    let part = Partition {
        exits: 1..3,
        ..fixture::partition(Search::Long, 4)
    };
    let mut cell = base_cell(Search::Long);
    cell[RowId::Exit.index()] = 1;
    let table = fixture::table_of(
        &generation,
        part,
        1,
        vec![(0, vec![(decision(cell, 0, 9, 40_000, 0), vec![5, 6])])],
    );
    struct Mixes(Vec<Option<MixKey>>);
    impl Visitor for Mixes {
        fn visit(&mut self, node: &Node<'_>) {
            assert_eq!(node.support(), 1);
            assert_eq!(node.table().len(), 1);
            self.0.extend((0..4).map(|v| node.mix(v)));
        }
        fn fork(&self) -> Self {
            Self(Vec::new())
        }
        fn absorb(&mut self, later: Self) {
            self.0.extend(later.0);
        }
    }
    let mut mixes = Mixes(Vec::new());
    walk(&table, 1, Schedule::Sequential, &mut mixes).unwrap();
    let root = table.root_key();
    assert_eq!(root.0[RowId::Exit.index()], OPEN);
    assert_eq!(
        mixes.0,
        vec![
            None,
            Some(with(root, RowId::Exit, 1)),
            Some(with(root, RowId::Exit, 2)),
            None
        ]
    );
    assert_eq!(mixes.fork().0.len(), 0);
}

/// Outcome records for a table whose decision `i` of `exits` variants uses
/// indices `i * exits + v`: priced trades, some unpriced, some losses.
fn outcomes_for(rows: u32, exits: u32, rng: &mut Seeded) -> Vec<Outcome> {
    (0..rows * exits)
        .map(|_| {
            let qty = 10 + rng.below(90);
            let entry = 1_000_000 + i128::from(rng.below(1_000) as u16);
            let price = |rng: &mut Seeded| {
                let gain = i128::from(rng.below(400) as u16) - 150;
                let second = 40_000 + rng.below(10_000) as u32;
                Slot::Priced(fixture::priced(
                    qty,
                    entry * i128::from(qty),
                    (entry + gain) * i128::from(qty),
                    100,
                    40_000,
                    second,
                ))
            };
            let slot = |rng: &mut Seeded| match rng.below(10) {
                0 => Slot::Unpriced(runner::fib::score::Unpriced::FeedHole),
                1 => Slot::ZeroQty,
                _ => price(rng),
            };
            Outcome {
                slots: [slot(rng), slot(rng)],
            }
        })
        .collect()
}

/// The batches a parallel walk splits, found from the visited nodes alone:
/// each (node, free row) whose node has at least `split_support` rows and two
/// or more visited children fixing that row, with the scratch bytes the
/// specification gives it (each child's rows times the free rows still open
/// below it, plus the node's rows for a prefix child that scans its own,
/// times an index and a cell). Returns the tasks the batches within `budget`
/// spawn (one per child) and how many batches pass the budget and run inline.
fn batches(visited: &Collect, split_support: u64, budget: u64) -> (u64, u64, Vec<u64>) {
    let free = free_rows();
    let last_fixed = |key: &MixKey| {
        free.iter()
            .rev()
            .find(|id| key.0[id.index()] != OPEN)
            .copied()
    };
    let support: HashMap<MixKey, u64> = visited
        .nodes
        .iter()
        .map(|(key, rows, ..)| (*key, rows.len() as u64))
        .collect();
    let mut children: BTreeMap<(MixKey, usize), Vec<u64>> = BTreeMap::new();
    for (key, rows, ..) in &visited.nodes {
        if let Some(id) = last_fixed(key) {
            let parent = with(*key, id, OPEN);
            let position = free.iter().position(|f| *f == id).unwrap();
            children
                .entry((parent, position))
                .or_default()
                .push(rows.len() as u64);
        }
    }
    let entry = (size_of::<u32>() + size_of::<Coords>()) as u64;
    let (mut tasks, mut inline, mut sizes) = (0, 0, Vec::new());
    for ((parent, position), rows) in &children {
        let parent_rows = support[parent];
        if rows.len() < 2 || parent_rows < split_support {
            continue;
        }
        let below = (free.len() - position - 1) as u64;
        let own = if free[*position].row().kind == RowKind::Cumulative {
            parent_rows
        } else {
            0
        };
        let bytes: u64 = rows.iter().map(|r| (r * below + own) * entry).sum();
        sizes.push(bytes);
        if bytes > budget {
            inline += 1;
        } else {
            tasks += rows.len() as u64;
        }
    }
    (tasks, inline, sizes)
}

/// FM-F053: a work-stealing walk at 1, 4 and 12 threads hands every node to
/// forked visitors that, absorbed in visit order, equal the sequential walk
/// byte for byte — the scored cards included — and its tally is the same. A
/// batch over the budget runs inline, is counted, and changes no byte.
#[test]
fn work_stealing_equals_sequential_bytes() {
    let mut rng = Seeded(0x3EA1_0006);
    let exits = 2_u8;
    let rows = 2_500;
    let (generation, data) = shared_case(&mut rng, exits, rows);
    let table = fixture::table_of(
        &generation,
        fixture::partition(Search::Long, exits),
        3,
        vec![(0, data)],
    );
    let outcomes = outcomes_for(rows, u32::from(exits), &mut rng);
    let calendar = fixture::calendar(3_990, 60);
    let profiles = profiles_of(&table);
    let sink = || {
        CardSink::new(
            &table,
            &profiles,
            Rungs::new(Vec::new(), Vec::new()).unwrap(),
            vec![Gross; profiles.len()],
            &outcomes,
            &calendar,
        )
        .unwrap()
    };
    let mut sequential = sink();
    let walked = walk(&table, 6, Schedule::Sequential, &mut sequential).unwrap();
    assert!(
        sequential.refusals().is_empty(),
        "{:?}",
        sequential.refusals().first()
    );
    assert_eq!(sequential.mixes(), walked.tally.mixes(profiles.len()));
    assert_eq!(walked.spread.tasks, 0);
    let (collected, _) = collect(&table, 6);
    for threads in [1, 4, 12] {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .unwrap();
        let mut mixed = false;
        let (_, _, mut sizes) = batches(&collected, 40, u64::MAX);
        sizes.sort_unstable();
        let exact = sizes[sizes.len() / 2];
        for (split_support, budget) in [
            (40, u64::MAX),
            (1, u64::MAX),
            (40, 1),
            (40, 100_000),
            (40, 400_000),
            (1, 1_000_000),
            (40, exact),
            (40, exact - 1),
        ] {
            let schedule = Schedule::Parallel {
                split_support,
                batch_budget_bytes: budget,
            };
            let mut parallel = sink();
            let spread = pool.install(|| walk(&table, 6, schedule, &mut parallel).unwrap());
            assert_eq!(
                parallel.bytes(),
                sequential.bytes(),
                "{threads} threads, {schedule:?}"
            );
            assert_eq!(spread.tally, walked.tally);
            let (tasks, inline, _) = batches(&collected, split_support, budget);
            assert_eq!(
                (spread.spread.tasks, spread.spread.inline_for_budget),
                (tasks, inline),
                "{threads} threads, {schedule:?}"
            );
            mixed |= tasks > 0 && inline > 0;
            let mut nodes = Collect::default();
            pool.install(|| walk(&table, 6, schedule, &mut nodes).unwrap());
            assert_eq!(nodes, collected);
        }
        assert!(
            mixed,
            "a budget between the batch sizes splits some batches and inlines others"
        );
    }
}

/// FM-F052: a lone survivor equal to its parent — which an ordinary walk skips
/// as equivalent — is scored by the guided walk.
#[test]
fn deep_single_survivor_is_scored() {
    let generation = generation(Search::Long, 2, 2, 2);
    let data: Vec<_> = (0..20_u32)
        .map(|i| {
            let mut cell = base_cell(Search::Long);
            cell[RowId::Weekday.index()] = (i % 2) as u8;
            (
                decision(cell, i, 50, 40_000, u64::from(i)),
                vec![2 * i, 2 * i + 1],
            )
        })
        .collect();
    let table = fixture::table_of(
        &generation,
        fixture::partition(Search::Long, 2),
        1,
        vec![(0, data)],
    );
    let survivor = with(with(table.root_key(), RowId::FibZone, 0), RowId::Exit, 1);
    let (ordinary, _) = collect(&table, 1);
    assert!(
        ordinary
            .nodes
            .iter()
            .all(|(k, ..)| *k != with(survivor, RowId::Exit, OPEN))
    );
    let survivors = Survivors::new(&table, &[survivor]).unwrap();
    assert_eq!(survivors.len(), 1);
    assert!(!survivors.is_empty());
    let mut guided = Collect::default();
    let tally = walk_guided(&table, &survivors, &mut guided);
    assert_eq!(tally.scored, 1);
    assert_eq!(tally.passed_through, 1, "the root, on the way");
    assert_eq!(tally.row_touches, 20);
    assert_eq!(guided.nodes.len(), 1);
    let (key, rows, fixed, wanted) = &guided.nodes[0];
    assert_eq!(with(*key, RowId::Exit, 1), survivor);
    assert_eq!(rows.len(), 20);
    assert_eq!(*fixed, 1);
    assert_eq!(wanted.as_deref(), Some(&[1][..]));
    let outcomes: Vec<Outcome> = (0..40)
        .map(|_| Outcome {
            slots: [Slot::Inapplicable, Slot::Inapplicable],
        })
        .collect();
    let calendar = fixture::calendar(40, 30);
    let profiles = profiles_of(&table);
    let mut sink = CardSink::new(
        &table,
        &profiles,
        Rungs::new(Vec::new(), Vec::new()).unwrap(),
        vec![Gross; profiles.len()],
        &outcomes,
        &calendar,
    )
    .unwrap();
    let _ = walk_guided(&table, &survivors, &mut sink);
    assert_eq!(
        sink.mixes(),
        2,
        "the survivor's variant at both sizes, and nothing else"
    );
    assert_eq!(&sink.bytes()[..ROW_COUNT], &survivor.0);
}

/// FM-F052: the guided walk scores exactly the survivors, each on the rows
/// `contains` selects from the whole table, whatever their counts.
#[test]
fn guided_scoring_equals_direct_per_survivor_scoring_via_contains() {
    let mut rng = Seeded(0x6D1D_0006);
    let exits = 3_u8;
    let (generation, data) = shared_case(&mut rng, exits, 900);
    let table = fixture::table_of(
        &generation,
        fixture::partition(Search::Long, exits),
        1,
        vec![(0, data)],
    );
    let layout = *generation.layout();
    let free = free_rows();
    let cells: Vec<Coords> = (0..table.len() as u32)
        .map(|r| *table.row(r).unwrap().coords)
        .collect();
    let mut survivors = BTreeSet::new();
    while survivors.len() < 60 {
        let mut key = with(
            table.root_key(),
            RowId::Exit,
            rng.below(u64::from(exits)) as u8,
        );
        for _ in 0..rng.below(4) {
            let id = free[rng.below(free.len() as u64) as usize];
            if id == RowId::Side {
                continue;
            }
            key = with(key, id, rng.below(u64::from(layout.radix(id))) as u8);
        }
        survivors.insert(key);
    }
    let keys: Vec<MixKey> = survivors.iter().copied().collect();
    let trie = Survivors::new(&table, &keys).unwrap();
    let mut guided = Collect::default();
    let tally = walk_guided(&table, &trie, &mut guided);
    let mut scored = BTreeSet::new();
    for (key, rows, fixed, wanted) in &guided.nodes {
        let want: Vec<u32> = (0..cells.len() as u32)
            .filter(|r| contains(key, &cells[*r as usize]))
            .collect();
        assert_eq!(rows, &want);
        let bound = free.iter().filter(|id| key.0[id.index()] != OPEN).count();
        assert_eq!(*fixed, bound);
        for variant in wanted.as_ref().unwrap() {
            assert!(
                scored.insert(with(*key, RowId::Exit, *variant)),
                "scored once"
            );
        }
    }
    assert_eq!(scored, survivors);
    assert_eq!(tally.scored, guided.nodes.len() as u64);
    let verdicts: Vec<MixKey> = scored.iter().copied().collect();
    assert_eq!(reconcile(&keys, &verdicts), Ok(()));
}

/// FM-F052: `fib verify` fails when a survivor has no verdict, when a verdict
/// names no survivor, and when one survivor has two.
#[test]
fn verify_fails_when_a_survivor_has_no_deep_verdict() {
    let generation = generation(Search::Long, 2, 2, 2);
    let table = fixture::table_of(&generation, fixture::partition(Search::Long, 2), 1, vec![]);
    let a = with(table.root_key(), RowId::Exit, 0);
    let b = with(table.root_key(), RowId::Exit, 1);
    assert_eq!(
        reconcile(&[a, b], &[a]),
        Err(LatticeRefusal::SurvivorWithoutVerdict { key: b })
    );
    assert_eq!(
        reconcile(&[a], &[a, b]),
        Err(LatticeRefusal::VerdictWithoutSurvivor { key: b })
    );
    assert_eq!(
        reconcile(&[a, b], &[a, a]),
        Err(LatticeRefusal::VerdictRepeated { key: a })
    );
    assert_eq!(
        reconcile(&[a, a], &[a]),
        Err(LatticeRefusal::SurvivorRepeated { key: a })
    );
    assert_eq!(reconcile(&[], &[]), Ok(()));
    assert_eq!(reconcile(&[b, a], &[a, b]), Ok(()));
}

/// A survivor trie refuses, by name, a key the layout refuses, the other
/// direction, another partition, an exit the table does not score and a key
/// given twice.
#[test]
fn a_survivor_trie_refuses_by_name() {
    let generation = generation(Search::Long, 2, 2, 3);
    let part = Partition {
        exits: 0..2,
        ..fixture::partition(Search::Long, 3)
    };
    let table = fixture::table_of(&generation, part, 1, vec![]);
    let ok = with(table.root_key(), RowId::Exit, 0);
    assert!(matches!(
        Survivors::new(&table, &[table.root_key()]),
        Err(LatticeRefusal::Mix(CubeRefusal::OpenPartitionRow {
            row: "exit"
        }))
    ));
    assert_eq!(
        Survivors::new(&table, &[with(ok, RowId::Side, 1)]),
        Err(LatticeRefusal::SideOutsideSearch {
            search: "long",
            side: 1
        })
    );
    assert_eq!(
        Survivors::new(&table, &[with(ok, RowId::EntryStyle, 2)]),
        Err(LatticeRefusal::PartitionMismatch {
            row: "entry_style",
            want: 0,
            have: 2
        })
    );
    assert_eq!(
        Survivors::new(&table, &[with(ok, RowId::Exit, 2)]),
        Err(LatticeRefusal::SurvivorExitOutside { variant: 2 })
    );
    assert_eq!(
        Survivors::new(&table, &[ok, ok]),
        Err(LatticeRefusal::SurvivorRepeated { key: ok })
    );
    let empty = Survivors::new(&table, &[]).unwrap();
    assert!(empty.is_empty());
    let mut guided = Collect::default();
    let tally = walk_guided(&table, &empty, &mut guided);
    assert_eq!((tally.scored, tally.passed_through), (0, 1));
}

/// Every refusal name is distinct and printed.
#[test]
fn every_lattice_refusal_is_named_once() {
    let key = MixKey([OPEN; ROW_COUNT]);
    let all = [
        LatticeRefusal::MinSupportZero,
        LatticeRefusal::PartitionValueOutOfRange {
            row: "r",
            value: 1,
            values: 1,
        },
        LatticeRefusal::ExitsInvalid {
            start: 0,
            end: 0,
            values: 1,
        },
        LatticeRefusal::MonthsZero,
        LatticeRefusal::MonthOutOfRange {
            month: 1,
            months: 1,
        },
        LatticeRefusal::MonthNotAfterLast { month: 1, last: 1 },
        LatticeRefusal::MonthDaysOverlap {
            month: 1,
            first_day: 1,
            previous_last_day: 1,
        },
        LatticeRefusal::OutcomesLength {
            decisions: 1,
            variants: 1,
            outcomes: 0,
        },
        LatticeRefusal::Cell(CubeRefusal::EmptySample),
        LatticeRefusal::Mix(CubeRefusal::EmptySample),
        LatticeRefusal::SideOutsideSearch {
            search: "long",
            side: 1,
        },
        LatticeRefusal::PartitionMismatch {
            row: "r",
            want: 0,
            have: 1,
        },
        LatticeRefusal::SecondOutOfDay { second: 1 },
        LatticeRefusal::InputNotInSlotOrder { at: 1 },
        LatticeRefusal::RowsExceedU32,
        LatticeRefusal::SurvivorExitOutside { variant: 1 },
        LatticeRefusal::SurvivorRepeated { key },
        LatticeRefusal::SurvivorWithoutVerdict { key },
        LatticeRefusal::VerdictWithoutSurvivor { key },
        LatticeRefusal::VerdictRepeated { key },
    ];
    let names: BTreeSet<&str> = all.iter().map(LatticeRefusal::name).collect();
    assert_eq!(names.len(), all.len());
    for refusal in &all {
        let shown = refusal.to_string();
        assert!(shown.starts_with("fib lattice refused ("), "{shown}");
        assert!(shown.contains(refusal.name()));
        let _: &dyn std::error::Error = refusal;
    }
}

/// The words a source-shape scan refuses, found as whole words, case aside.
fn refused_words<'a>(source: &str, words: &[&'a str]) -> Vec<&'a str> {
    let tokens: BTreeSet<String> = source
        .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .map(str::to_ascii_lowercase)
        .collect();
    words
        .iter()
        .copied()
        .filter(|w| tokens.contains(*w))
        .collect()
}

/// `CLAUDE.md` §6: `lattice.rs` names no depth, `k` or `max_level` anywhere —
/// its types carry no such field and no parameter bounds how far it grows.
#[test]
fn the_lattice_names_no_depth_k_or_max_level() {
    let words = ["depth", "k", "max_level"];
    assert_eq!(
        refused_words(include_str!("../src/fib/lattice.rs"), &words),
        Vec::<&str>::new()
    );
    assert_eq!(
        refused_words("fn walk(max_depth: u8, k: u32) // Depth-first", &words),
        vec!["depth", "k"],
        "the scan sees what it refuses"
    );
}

/// NO TYPED NUMBERS (operator 16 Oct 11:24 IST): FIB-6's files hold no
/// numeric literal but 0 and 1 in their shipping part.
#[test]
fn fib6_files_type_no_number() {
    for (name, source) in [
        ("lattice.rs", include_str!("../src/fib/lattice.rs")),
        ("score.rs", include_str!("../src/fib/score.rs")),
    ] {
        assert_eq!(untyped_numbers(source), Vec::new(), "{name}");
    }
}

/// `expand_range` is one node's expansion: on the whole table it visits the
/// root and exactly the walk's frequent children of the root, with the
/// walk's classification of them; on a sub-range each child holds the range's
/// rows that `contains` selects. Its refusals are by name.
#[test]
fn expand_range_is_one_nodes_expansion() {
    use runner::fib::lattice::{Scratch, expand_range};
    let mut rng = Seeded(0xE4A9_0006);
    for _ in 0..8 {
        let case = random_case(&mut rng, Search::Long, 1_500);
        let min = 1 + case.min_support / 5;
        let (walked, walk_tally) = collect(&case.table, min);
        let rows = case.table.len() as u32;
        let mut scratch = Scratch::for_rows(rows as usize);
        assert_eq!(scratch.rows(), rows as usize);
        let mut once = Collect::default();
        let tally = expand_range(&case.table, 0..rows, min, &mut scratch, &mut once).unwrap();
        let first: Vec<_> = walked.nodes.iter().filter(|n| n.2 <= 1).cloned().collect();
        if u64::from(rows) >= min {
            assert_eq!(once.nodes, first);
            for slot in 0..2 {
                assert_eq!(tally.frequent[slot], walk_tally.frequent[slot]);
                assert_eq!(tally.infrequent[slot], walk_tally.infrequent[slot]);
                assert_eq!(
                    tally.equivalent_to_parent[slot],
                    walk_tally.equivalent_to_parent[slot]
                );
                assert_eq!(
                    tally.equivalent_to_sibling[slot],
                    walk_tally.equivalent_to_sibling[slot]
                );
            }
            assert_eq!(tally.frequent[2..], [0; FIXED_SLOTS - 2]);
        }
        let (start, end) = (rows / 3, rows - rows / 4);
        let mut part = Collect::default();
        expand_range(&case.table, start..end, 1, &mut scratch, &mut part).unwrap();
        for (key, node_rows, fixed, _) in &part.nodes {
            assert!(*fixed <= 1);
            let want: Vec<u32> = (start..end)
                .filter(|r| contains(key, &case.cells[*r as usize]))
                .collect();
            assert_eq!(node_rows, &want);
        }
    }
    let generation = generation(Search::Long, 2, 2, 1);
    let data = (0..10_u32)
        .map(|i| {
            (
                decision(base_cell(Search::Long), i, 5, 40_000, u64::from(i)),
                vec![i],
            )
        })
        .collect();
    let table = fixture::table_of(
        &generation,
        fixture::partition(Search::Long, 1),
        1,
        vec![(0, data)],
    );
    let mut scratch = Scratch::for_rows(4);
    let mut sink = Collect::default();
    assert_eq!(
        expand_range(&table, 0..4, 0, &mut scratch, &mut sink).unwrap_err(),
        LatticeRefusal::MinSupportZero
    );
    assert_eq!(
        expand_range(&table, 8..11, 1, &mut scratch, &mut sink).unwrap_err(),
        LatticeRefusal::RangeOutsideTable {
            start: 8,
            end: 11,
            rows: 10
        }
    );
    #[allow(
        clippy::reversed_empty_ranges,
        reason = "the backwards range is the case under test"
    )]
    let backwards = 5..2;
    assert_eq!(
        expand_range(&table, backwards, 1, &mut scratch, &mut sink).unwrap_err(),
        LatticeRefusal::RangeOutsideTable {
            start: 5,
            end: 2,
            rows: 10
        }
    );
    assert_eq!(
        expand_range(&table, 0..5, 1, &mut scratch, &mut sink).unwrap_err(),
        LatticeRefusal::ScratchTooSmall { needed: 5, have: 4 }
    );
    let tally = expand_range(&table, 0..4, 1, &mut scratch, &mut sink).unwrap();
    assert_eq!((tally.frequent[0], tally.nodes()), (1, 1));
    assert_eq!(
        tally.row_touches,
        4 * runner::fib::lattice::FREE_ROWS as u64,
        "one histogram pass per free row, no scatter"
    );
    assert_eq!(
        runner::fib::lattice::FREE_ROWS,
        21,
        "24 rows less the three partition rows"
    );
    assert_eq!(
        FIXED_SLOTS, 25,
        "one slot per possible fixed-row count, 0 to 24"
    );
    assert!(sink.nodes.len() == 1 && sink.nodes[0].1 == vec![0, 1, 2, 3]);
    let refusals = [
        LatticeRefusal::RangeOutsideTable {
            start: 0,
            end: 1,
            rows: 0,
        },
        LatticeRefusal::ScratchTooSmall { needed: 1, have: 0 },
    ];
    assert_eq!(
        refusals.map(|r| r.name()),
        ["range_outside_table", "scratch_too_small"]
    );
}

/// A mix the scoring pass refuses is recorded by the sink, by key and by
/// name, and adds no card; `expand_range` below its floor counts its node
/// infrequent and visits nothing.
#[test]
fn a_refused_mix_is_recorded_and_a_range_below_its_floor_visits_nothing() {
    use runner::fib::lattice::{Scratch, expand_range};
    use runner::fib::score::ScoreRefusal;
    let generation = generation(Search::Long, 2, 2, 1);
    let data = (0..6_u32)
        .map(|i| {
            (
                decision(base_cell(Search::Long), i, 60, 40_000, u64::from(i)),
                vec![i],
            )
        })
        .collect();
    let table = fixture::table_of(
        &generation,
        fixture::partition(Search::Long, 1),
        1,
        vec![(0, data)],
    );
    let short: Vec<Outcome> = (0..3)
        .map(|_| Outcome {
            slots: [Slot::ZeroQty, Slot::ZeroQty],
        })
        .collect();
    let calendar = fixture::calendar(50, 30);
    let profiles = profiles_of(&table);
    let mut sink = CardSink::new(
        &table,
        &profiles,
        Rungs::new(Vec::new(), Vec::new()).unwrap(),
        vec![Gross; profiles.len()],
        &short,
        &calendar,
    )
    .unwrap();
    walk(&table, 1, Schedule::Sequential, &mut sink).unwrap();
    assert_eq!(sink.mixes(), 0);
    assert!(sink.bytes().is_empty());
    assert_eq!(
        sink.refusals(),
        &[(
            table.root_key(),
            ScoreRefusal::OutcomeOutOfRange {
                index: 3,
                outcomes: 3
            }
        )]
    );
    let mut scratch = Scratch::for_rows(6);
    let mut visited = Collect::default();
    let tally = expand_range(&table, 0..4, 5, &mut scratch, &mut visited).unwrap();
    assert!(visited.nodes.is_empty());
    assert_eq!(
        (tally.infrequent[0], tally.frequent[0], tally.row_touches),
        (1, 0, 0)
    );
}
