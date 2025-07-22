import { Component, ErrorInfo, ReactNode } from "react";
import Error from "./Error";
import { RemailsError } from "./error";

interface Props {
  children?: ReactNode;
}

interface State {
  error?: Error;
}

export default class ErrorBoundary extends Component<Props, State> {
  public state: State = {
    error: undefined,
  };

  public static getDerivedStateFromError(e: Error): State {
    // Update state so the next render will show the fallback UI.
    return { error: e };
  }

  public componentDidCatch(error: Error, errorInfo: ErrorInfo) {
    console.error("Uncaught error:", error, errorInfo);
  }

  public render() {
    if (this.state.error) {
      if (this.state.error instanceof RemailsError) {
        return <Error error={this.state.error} />;
      }

      return <h1>Sorry.. there was an error</h1>;
    }

    return this.props.children;
  }
}
