import { Suspense, lazy } from "react";
import { Route, Routes } from "react-router-dom";

const AccountView = lazy(() => import(/* webpackPrefetch: true */ "./views/Account"));

export const AppRouter: React.FC = () => {
  return (
    <Suspense fallback={<div>Loading...</div>}>
      <Routes>
        <Route path="accounts/:index" element={<AccountView />} />
      </Routes>
    </Suspense>
  );
};
