import { useCallback, useEffect, useState } from "react";
import { localUsage, onUsageUpdated, rescan } from "../api";
import type { LocalUsage, Range, Tool } from "../types";

const tools: Array<[Tool, string]> = [["claude-code", "Claude Code"], ["codex", "Codex"], ["gemini-cli", "Gemini CLI"]];
const ranges: Array<[Range, string]> = [["today", "오늘"], ["week", "7일"], ["all", "전체"]];
const number = new Intl.NumberFormat("ko-KR");
const compact = (value: number) => value < 1000 ? String(value) : value < 1_000_000 ? `${(value / 1000).toFixed(1)}K` : `${(value / 1_000_000).toFixed(1)}M`;
const clock = (value: string | null) => value ? new Intl.DateTimeFormat("ko-KR", { hour: "2-digit", minute: "2-digit", second: "2-digit", hour12: false }).format(new Date(value)) : "-";

function Sparkline({ usage }: { usage: LocalUsage }) {
  const values = usage.daily.map(({ totalTokens }) => totalTokens);
  const max = Math.max(...values, 1);
  const points = values.map((value, index) => `${index * (160 / Math.max(values.length - 1, 1))},${44 - value / max * 38}`).join(" ");
  return <svg className="sparkline" viewBox="0 0 160 48" aria-label="최근 7일 사용량"><polyline points={points} /></svg>;
}

export function UsagePanel({ onSettings }: { onSettings: () => void }) {
  const [range, setRange] = useState<Range>("today");
  const [usage, setUsage] = useState<LocalUsage>();
  const [error, setError] = useState("");
  const load = useCallback(async () => {
    try { setUsage(await localUsage(range)); setError(""); } catch (reason) { setError(String(reason)); }
  }, [range]);
  useEffect(() => { void load(); }, [load]);
  useEffect(() => { let unlisten: (() => void) | undefined; void onUsageUpdated(load).then(stop => { unlisten = stop; }); return () => unlisten?.(); }, [load]);
  const refresh = async () => { try { setUsage(await rescan()); setError(""); } catch (reason) { setError(String(reason)); } };
  const maxTool = Math.max(...(usage?.byTool.map(item => item.counter.totalTokens) ?? [0]), 1);

  return <main className="panel">
    <header><strong>디지몬 어드벤처</strong><span><button onClick={onSettings} aria-label="설정">⚙</button><button onClick={() => void refresh()} aria-label="새로고침">↻</button></span></header>
    <nav className="tabs">{ranges.map(([value, label]) => <button key={value} className={range === value ? "active" : ""} onClick={() => setRange(value)}>{label}</button>)}</nav>
    {error ? <p className="message">{error}</p> : !usage ? <p className="message">불러오는 중…</p> : <>
      {usage.total.records === 0 ? <p className="empty">아직 사용 기록이 없습니다</p> : <>
        <section className="total"><b>{number.format(usage.total.totalTokens)}</b><span>토큰</span><Sparkline usage={usage} /></section>
        <section className="breakdown">{[["입력", usage.total.inputTokens], ["출력", usage.total.outputTokens], ["캐시 쓰기", usage.total.cacheWriteTokens], ["캐시 읽기", usage.total.cacheReadTokens]].map(([label, value]) => <div key={String(label)}><span>{label}</span><strong>{number.format(Number(value))}</strong></div>)}</section>
        <section><h2>도구별</h2>{tools.map(([tool, label]) => { const value = usage.byTool.find(item => item.tool === tool)?.counter.totalTokens ?? 0; return <div className="tool-row" key={tool}><span>{label}</span><b>{compact(value)}</b><i><em style={{ width: `${value / maxTool * 100}%` }} /></i></div>; })}</section>
        <section><h2>모델별</h2><div className="models">{usage.byModel.map(item => <div key={`${item.tool}-${item.model}`}><span>{item.model}</span><b>{compact(item.counter.totalTokens)}</b></div>)}</div></section>
      </>}
      <footer>마지막 스캔 {clock(usage.lastScanAt)}{usage.scanning ? " · 스캔 중" : ""}</footer>
    </>}
  </main>;
}
