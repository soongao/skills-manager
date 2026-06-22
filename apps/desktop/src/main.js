import "./styles.css";

import { createDesktopApi } from "./desktop-api.js";
import { bindView } from "./events.js";
import { createViewModel } from "./view-model.js";
import { renderSettings } from "./views/settings.js";

const app = document.querySelector("#app");
const api = createDesktopApi();

const state = {
  dashboard: null,
  busy: false,
  error: null,
  lastRemoteSync: null,
  settingsPage: "overview",
  selectedSkillId: null,
};

async function refresh() {
  state.busy = true;
  state.error = null;
  render();

  try {
    state.dashboard = await api.loadDashboard();
  } catch (error) {
    state.error = api.normalizeError(error);
  } finally {
    state.busy = false;
    render();
  }
}

async function action(fn) {
  state.busy = true;
  state.error = null;
  render();

  try {
    const result = await fn();
    if (result?.sync || result?.plan || result?.remoteLink || result?.remoteLinkPlan) {
      state.lastRemoteSync = result;
      state.settingsPage = "activity";
      state.selectedSkillId = null;
    }
    state.dashboard = await api.loadDashboard();
  } catch (error) {
    state.error = api.normalizeError(error);
  } finally {
    state.busy = false;
    render();
  }
}

function render() {
  const model = createViewModel(state.dashboard);
  app.innerHTML = renderSettings(model, state);

  bindView({
    action,
    api,
    refresh,
    render,
    root: app,
    state,
  });
}

render();
refresh();
