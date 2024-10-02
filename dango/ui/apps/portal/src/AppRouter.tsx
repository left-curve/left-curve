import { lazy } from "react";
import { Route, Routes } from "react-router-dom";
import { Layout } from "./components/Layout";

const AccountView = lazy(() => import(/* webpackPrefetch: true */ "./views/Account"));

export const AppRouter: React.FC = () => {
  return (
    <Routes>
      <Route path="/" element={<Layout />}>
        <Route path="account/:index" element={<AccountView />} />
      </Route>
    </Routes>
  );
};
