import { ChevronDown, List } from "lucide-react";

export default function DeployHistoryCard({ versions, onLoad, onPick }) {
  return (
    <section className="card" style={{ "--i": 2 }}>
      <div className="card-head">
        <h2>Version cũ hơn</h2>
        <span className="card-sub">DeployHistory</span>
      </div>
      <div className="btn-row">
        <button id="btn-load-history" className="btn small" onClick={onLoad}>
          <List className="ic" />
          Nạp 30 bản gần đây
        </button>
      </div>
      <div className="select-wrap">
        <select
          id="history-select"
          className="select"
          defaultValue=""
          onChange={(e) => {
            const v = e.target.value;
            if (v) onPick(v);
          }}
        >
          <option value="">— chưa nạp —</option>
          {versions.map((v) => (
            <option value={v} key={v}>
              {v}
            </option>
          ))}
        </select>
        <ChevronDown className="select-caret" />
      </div>
    </section>
  );
}
