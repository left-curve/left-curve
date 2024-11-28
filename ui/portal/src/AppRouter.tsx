import { Spinner } from "@dango/shared";
import { Suspense, lazy } from "react";
import { Route, Routes } from "react-router-dom";
import { AppLayout } from "./components/AppLayout";
import { AuthLayout } from "./components/AuthLayout";

import { NotFoundView } from "./views/NotFound";

// Auth routes
const LoginView = lazy(() => import(/* webpackPrefetch: true */ "./views/Login"));
const SignupView = lazy(() => import(/* webpackPrefetch: true */ "./views/Signup"));

// Portal routes
const AccountView = lazy(() => import(/* webpackPrefetch: true */ "./views/Account"));
const TransferView = lazy(() => import(/* webpackPrefetch: true */ "./views/Transfer"));
const SwapView = lazy(() => import(/* webpackPrefetch: true */ "./views/Swap"));
const PoolView = lazy(() => import(/* webpackPrefetch: true */ "./views/Pool"));
const BlockExplorerView = lazy(() => import(/* webpackPrefetch: true */ "./views/BlockExplorer"));
const AccountCreationView = lazy(
  () => import(/* webpackPrefetch: true */ "./views/AccountCreation"),
);

export const AppRouter: React.FC = () => {
  return (
    <Suspense
      fallback={
        <div className="h-screen w-full flex justify-center items-center">
          <Spinner size="lg" color="pink" />
        </div>
      }
    >
      <Routes>
        <Route path="/auth" element={<AuthLayout />}>
          <Route path="login" element={<LoginView />} />
          <Route path="signup" element={<SignupView />} />
        </Route>
        <Route path="/" element={<AppLayout />}>
          <Route path="accounts" element={<AccountView />} />
          <Route path="/account-creation" element={<AccountCreationView />} />
          <Route path="/block-explorer" element={<BlockExplorerView />} />
          <Route path="/transfer" element={<TransferView />} />
          <Route path="/swap" element={<SwapView />} />
          <Route path="/amm" element={<PoolView />} />
          <Route path="*" element={<NotFoundView />} />
        </Route>
      </Routes>
    </Suspense>
  );
};
