import { Component, useEffect, type ReactNode } from "react";

interface AppErrorBoundaryProps {
  readonly children: ReactNode;
}

interface AppErrorBoundaryState {
  readonly failed: boolean;
}

export function SafeApplicationError() {
  useEffect(() => {
    document.title = "Application error · Fruitboard";
  }, []);

  return (
    <main className="fatal-error">
      <section
        aria-labelledby="fatal-error-title"
        className="fatal-error__panel"
        role="alert"
      >
        <p className="eyebrow">Application error</p>
        <h1 id="fatal-error-title">Fruitboard needs to restart</h1>
        <p>The current view stopped safely. Restart Fruitboard to continue.</p>
      </section>
    </main>
  );
}

export class AppErrorBoundary extends Component<
  AppErrorBoundaryProps,
  AppErrorBoundaryState
> {
  override state: AppErrorBoundaryState = { failed: false };

  static getDerivedStateFromError(): AppErrorBoundaryState {
    return { failed: true };
  }

  override render() {
    if (this.state.failed) {
      return <SafeApplicationError />;
    }

    return this.props.children;
  }
}
