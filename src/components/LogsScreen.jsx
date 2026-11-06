import LogPanel from "./LogPanel.jsx";

export default function LogsScreen({ logs }) {
  return (
    <div className="screen logs-screen">
      <LogPanel logs={logs} />
    </div>
  );
}
