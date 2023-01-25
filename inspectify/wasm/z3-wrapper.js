/// @ts-check
/// <reference path="../ui/src/z3-wrapper.d.ts" />

export const init_context = async () => window.__z3Init();
export const run = async (ctx, cmd) => window.__z3Run(ctx, cmd);
