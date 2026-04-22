import { RUNTIME_TYPES_SCHEMA_VERSION } from "@terminal-platform/runtime-types";

export const WORKSPACE_CONTRACTS_SCHEMA_VERSION = 1 as const;

export const WORKSPACE_CONTRACTS_COMPATIBILITY = {
  schemaVersion: WORKSPACE_CONTRACTS_SCHEMA_VERSION,
  runtimeTypesSchemaVersion: RUNTIME_TYPES_SCHEMA_VERSION,
} as const;

export type WorkspaceContractsCompatibility = typeof WORKSPACE_CONTRACTS_COMPATIBILITY;
