import { Navigate, Route, Routes } from "react-router-dom";
import { AppShell } from "./app-shell";
import { GatewaysPage } from "../features/gateways/gateways-page";
import { SettingsPage } from "../features/settings/settings-page";
import { CodingPage } from "../features/workspace/coding-page";
import { DialogProvider } from "../shared/ui/dialog/dialog-provider";

export function App() {
  return (
    <DialogProvider>
      <Routes>
        <Route element={<AppShell />}>
          <Route index element={<CodingPage />} />
          <Route path="settings" element={<SettingsPage />} />
          <Route path="gateways" element={<GatewaysPage />} />
          <Route path="*" element={<Navigate to="/" replace />} />
        </Route>
      </Routes>
    </DialogProvider>
  );
}
