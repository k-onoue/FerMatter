import { Component, type ReactNode } from "react";

/** Never render exception text: project source can appear in parser errors. */
export class RecoveryBoundary extends Component<
  { children: ReactNode },
  { failed: boolean }
> {
  state = { failed: false };
  static getDerivedStateFromError() {
    return { failed: true };
  }
  render() {
    if (!this.state.failed) return this.props.children;
    return (
      <main className="empty" role="alert">
        <h1>Unable to display this view</h1>
        <p>
          Your project files have not been removed. Reload to reopen the app.
        </p>
        <button onClick={() => location.reload()}>Reload view</button>
      </main>
    );
  }
}
