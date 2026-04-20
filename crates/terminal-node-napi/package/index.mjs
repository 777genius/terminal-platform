import { createRequire } from "node:module";

const require = createRequire(import.meta.url);
const cjs = require("./index.cjs");

export const applyScreenDelta = cjs.applyScreenDelta;
export const createSessionState = cjs.createSessionState;
export const loadNativeBinding = cjs.loadNativeBinding;
export const reduceSessionWatchEvent = cjs.reduceSessionWatchEvent;
export const resolveNativeBindingPath = cjs.resolveNativeBindingPath;
export const TerminalNodeClient = cjs.TerminalNodeClient;
export const TerminalNodeSubscription = cjs.TerminalNodeSubscription;
export default cjs;
