import { AGENTS } from "../constants.js";
import { escapeAttr, escapeHtml, shortPath } from "../format.js";
import { isProblemStatus } from "../view-model.js";

export function renderPanel(model, state) {
  return `
    <section class="panel-shell">
      <header class="panel-header">
        <div class="app-glyph">SM</div>
        <div>
          <p>Skills Manager</p>
          <h1>${model.configured ? model.healthLabel : "Not configured"}</h1>
        </div>
        <span class="panel-state ${state.busy ? "working" : model.conflictCount ? "bad" : "good"}">${state.busy ? "Syncing" : model.conflictCount ? "Review" : "Ready"}</span>
      </header>

      ${state.error ? `<div class="issue compact">${escapeHtml(state.error)}</div>` : ""}

      ${model.configured ? renderPanelStatus(model, state) : renderPanelSetup()}
    </section>
  `;
}

function renderPanelStatus(model, state) {
  return `
    <section class="panel-source">
      <span>Shared source</span>
      <strong title="${escapeAttr(model.sourceRoot)}">${escapeHtml(shortPath(model.sourceRoot))}</strong>
    </section>

    <section class="panel-metrics">
      ${renderPanelMetric("Skills", model.skills.length)}
      ${renderPanelMetric("Links", model.enabledCount)}
      ${renderPanelMetric("Conflicts", model.conflictCount, model.conflictCount ? "bad" : "")}
    </section>

    <section class="panel-agent-list">
      ${AGENTS.map(([agentId, label]) => renderPanelAgent(model, agentId, label)).join("")}
    </section>

    <section class="panel-actions">
      <button class="mac-button primary" data-action="reconcile-all" ${state.busy ? "disabled" : ""}>Apply</button>
      <button class="mac-button" data-action="refresh" ${state.busy ? "disabled" : ""}>Refresh</button>
      <button class="mac-button" data-action="open-settings">Settings</button>
      <button class="mac-button quiet" data-action="hide-window">Hide</button>
    </section>
  `;
}

function renderPanelMetric(label, value, tone = "") {
  return `
    <div class="panel-metric ${tone}">
      <strong>${escapeHtml(value)}</strong>
      <span>${escapeHtml(label)}</span>
    </div>
  `;
}

function renderPanelSetup() {
  return `
    <section class="empty-panel">
      <span class="app-glyph large">SM</span>
      <h2>No shared source yet</h2>
      <p>Open settings and choose the folder that stores your skills.</p>
      <button class="mac-button primary" data-action="open-settings">Open settings</button>
    </section>
  `;
}

function renderPanelAgent(model, agentId, label) {
  const agent = model.agents.find((item) => item.agentId === agentId);
  const rows = model.statuses.filter((item) => item.agentId === agentId);
  const conflicts = rows.filter((item) => isProblemStatus(item.status)).length;
  const enabled = rows.filter((item) => item.status === "enabled").length;
  const managed = agent?.managed !== false && Boolean(agent);
  const tone = conflicts ? "bad" : managed ? "good" : "muted";

  return `
    <div class="panel-agent">
      <span class="state-light ${tone}"></span>
      <div>
        <strong>${escapeHtml(label)}</strong>
        <small>${managed ? `${enabled} linked` : "Not managed"}</small>
      </div>
      <span class="agent-count ${conflicts ? "bad" : ""}">${conflicts || enabled}</span>
    </div>
  `;
}
