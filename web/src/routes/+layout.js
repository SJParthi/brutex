// FULLY CLIENT-RENDERED, PRERENDERED SHELL.
//
// The Rust binary serves static files and answers JSON; it does not run a
// JavaScript renderer, and D-0053 forbids it ever needing to. So the shell is
// prerendered at build time and every route hydrates in the browser.
export const prerender = true;
export const ssr = false;
