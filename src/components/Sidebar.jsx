import { Gamepad2, House, ScrollText } from "lucide-react";

const TABS = [
  { id: "home", label: "Home", Icon: House },
  { id: "roblox", label: "Roblox", Icon: Gamepad2 },
  { id: "logs", label: "Logs", Icon: ScrollText },
];

export default function Sidebar({ active, onSelect }) {
  return (
    <aside className="sidebar">
      <div className="sidebar-logo">
        <span className="brand">
          VERLOX<span className="brand-cursor">_</span>
        </span>
        <span className="sidebar-version">v1.2.0</span>
      </div>
      <nav className="sidebar-nav">
        {TABS.map(({ id, label, Icon }) => (
          <button
            key={id}
            className={active === id ? "tab-btn active" : "tab-btn"}
            onClick={() => onSelect(id)}
          >
            <Icon className="ic" />
            <span>{label}</span>
          </button>
        ))}
      </nav>
    </aside>
  );
}
