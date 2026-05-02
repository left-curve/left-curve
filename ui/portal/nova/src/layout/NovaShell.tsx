import { View } from "react-native";
import { Outlet } from "@tanstack/react-router";
import { Header } from "./Header";
import { SearchPalette } from "./SearchPalette";
import { useSearch } from "./useSearch";
import { AuthProvider } from "../auth";

export function NovaShell() {
  // Mounts the global Cmd+K / Ctrl+K listener
  useSearch();

  return (
    <AuthProvider>
      <View className="flex-1 min-h-screen bg-bg-app">
        <Header />
        <View className="flex-1">
          <Outlet />
        </View>
        <SearchPalette />
      </View>
    </AuthProvider>
  );
}
