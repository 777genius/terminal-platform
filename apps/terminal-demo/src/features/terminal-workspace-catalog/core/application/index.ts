export interface TerminalWorkspaceCatalogFormState {
  title: string;
  program: string;
  args: string;
  cwd: string;
}

export const initialTerminalWorkspaceCatalogFormState: TerminalWorkspaceCatalogFormState = {
  title: "Workspace",
  program: "",
  args: "",
  cwd: "",
};
