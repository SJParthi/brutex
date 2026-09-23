import { ask } from './ask.js';
import { createInspectionReader, INITIAL_INSPECTION } from './runtime-inspection.js';

export const runtimeInspection = $state({ value: INITIAL_INSPECTION });
const reader = createInspectionReader(ask, value => { runtimeInspection.value = value; });
export const loadInspection = reader.load;
