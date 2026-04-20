import { createRequire } from "node:module";

const require = createRequire(import.meta.url);
const cjs = require("./index.cjs");

export const loadNativeBinding = cjs.loadNativeBinding;
export const resolveNativeBindingPath = cjs.resolveNativeBindingPath;
export const TerminalNodeClient = cjs.TerminalNodeClient;
export default cjs;
