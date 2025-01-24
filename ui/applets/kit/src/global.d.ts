interface Window {
  ReactNativeWebView?: {
    postMessage: (payload: string) => void;
  };
}
