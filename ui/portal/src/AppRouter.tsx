import { Spinner } from "@dango/shared";
import { Suspense, lazy } from "react";
import { Route, Routes } from "react-router-dom";
import { AppLayout } from "./components/AppLayout";
import { AuthLayout } from "./components/AuthLayout";

// Auth routes
const LoginView = lazy(() => import(/* webpackPrefetch: true */ "./views/Login"));
const SignupView = lazy(() => import(/* webpackPrefetch: true */ "./views/Signup"));

// Portal routes
const AccountView = lazy(() => import(/* webpackPrefetch: true */ "./views/Account"));
const TransferView = lazy(() => import(/* webpackPrefetch: true */ "./views/Transfer"));
const SwapView = lazy(() => import(/* webpackPrefetch: true */ "./views/Swap"));

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
          <Route path="accounts/:index" element={<AccountView />} />ç
          <Route path="/transfer" element={<TransferView />} />
          <Route path="/swap" element={<SwapView />} />
        </Route>
      </Routes>
    </Suspense>
  );
};
