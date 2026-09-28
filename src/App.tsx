import { useState } from "react";
import { SettingsPanel } from "./components/SettingsPanel";
import { UsagePanel } from "./components/UsagePanel";

export default function App() {
  const [view, setView] = useState<"usage" | "settings">("usage");
  return view === "usage"
    ? <UsagePanel onSettings={() => setView("settings")} />
    : <SettingsPanel onBack={() => setView("usage")} />;
}
