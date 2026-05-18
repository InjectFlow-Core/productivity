import { invoke } from '@tauri-apps/api/core';
import { marked } from 'marked';

// ── View switcher ─────────────────────────────────────────────────────────────

function showView(id) {
  for (const el of document.querySelectorAll('.view')) el.hidden = true;
  document.getElementById(id).hidden = false;
}

// ── Clock (shared) ────────────────────────────────────────────────────────────

function startClock(clockId, dateId) {
  invoke('get_current_time').then((t) => { document.getElementById(clockId).textContent = t; });
  invoke('get_today_date').then((d) => {
    document.getElementById(dateId).textContent = new Date(d + 'T00:00:00').toLocaleDateString('en-US', {
      weekday: 'long', year: 'numeric', month: 'long', day: 'numeric',
    });
  });
  setInterval(() => invoke('get_current_time').then((t) => { document.getElementById(clockId).textContent = t; }), 10000);
}

// ── Morning lock ──────────────────────────────────────────────────────────────

const CATS = [
  { id: 'personal',  label: 'Personal',  color: 'var(--cat-personal)'  },
  { id: 'work',      label: '9-5',       color: 'var(--cat-work)'       },
  { id: 'freelance', label: 'Freelance', color: 'var(--cat-freelance)'  },
  { id: 'community', label: 'Community', color: 'var(--cat-community)'  },
];

function catMeta(id) {
  return CATS.find((c) => c.id === id) || CATS[0];
}

async function startMorningLock() {
  startClock('clock', 'date');

  // tasks = [{ text, category }]
  const tasks = [];
  const carryovers = [];
  let activeCat = 'personal';

  const taskInput  = document.getElementById('task-input');
  const addTaskBtn = document.getElementById('add-task-btn');
  const carryoverField = document.getElementById('carryover-field');
  const carryoverList = document.getElementById('carryover-list');
  const taskGroups = document.getElementById('task-groups');
  const submitBtn  = document.getElementById('morning-btn');

  taskInput.focus();

  try {
    carryovers.push(...(await invoke('get_carryover_candidates')).map((task) => ({ ...task, selected: true })));
  } catch (err) {
    console.error(err);
  }

  // Category picker
  document.getElementById('cat-picker').addEventListener('click', (e) => {
    const pill = e.target.closest('.cat-pill');
    if (!pill) return;
    activeCat = pill.dataset.cat;
    document.querySelectorAll('.cat-pill').forEach((p) => p.classList.toggle('active', p === pill));
    taskInput.focus();
  });

  function selectedCarryovers() {
    return carryovers.filter((task) => task.selected);
  }

  function updateSubmitState() {
    submitBtn.disabled = tasks.length + selectedCarryovers().length === 0;
  }

  function renderCarryovers() {
    carryoverField.hidden = carryovers.length === 0;
    carryoverList.innerHTML = '';

    carryovers.forEach((task, i) => {
      const meta = catMeta(task.category);
      const row = document.createElement('label');
      row.className = 'carryover-row' + (task.selected ? ' selected' : '');
      row.innerHTML = `
        <input type="checkbox" ${task.selected ? 'checked' : ''} />
        <span class="task-cat-dot" data-cat="${meta.id}"></span>
        <span class="carryover-text">${escHtml(task.text)}</span>
        <span class="carryover-cat">${meta.label}</span>`;

      row.querySelector('input').addEventListener('change', (e) => {
        carryovers[i].selected = e.target.checked;
        row.classList.toggle('selected', e.target.checked);
        updateSubmitState();
      });

      carryoverList.appendChild(row);
    });

    updateSubmitState();
  }

  function renderTasks() {
    taskGroups.hidden = tasks.length === 0;
    taskGroups.innerHTML = '';

    // Group by category, preserving CATS order
    CATS.forEach(({ id, label, color }) => {
      const group = tasks.filter((t) => t.category === id);
      if (group.length === 0) return;

      const groupEl = document.createElement('div');
      groupEl.className = 'task-group';
      groupEl.innerHTML = `<div class="task-group-label cat-group-label" data-cat="${id}">${label}</div>`;
      const listEl = document.createElement('div');
      listEl.className = 'task-list';

      group.forEach((task) => {
        const globalIdx = tasks.indexOf(task);
        const groupIdx  = group.indexOf(task);
        const row = document.createElement('div');
        row.className = 'task-row';
        row.draggable = true;
        row.innerHTML = `
          <span class="task-cat-dot" data-cat="${id}"></span>
          <span class="task-text">${escHtml(task.text)}</span>
          <div class="task-actions">
            <button type="button" class="task-action" data-action="up"     ${groupIdx === 0              ? 'disabled' : ''}>↑</button>
            <button type="button" class="task-action" data-action="down"   ${groupIdx === group.length-1 ? 'disabled' : ''}>↓</button>
            <button type="button" class="task-action danger" data-action="remove">×</button>
          </div>`;

        row.querySelector('.task-actions').addEventListener('click', (e) => {
          const action = e.target.dataset.action;
          if (!action) return;
          if (action === 'remove') { tasks.splice(globalIdx, 1); renderTasks(); return; }
          // move within the same category bucket
          const swapWith = action === 'up' ? group[groupIdx - 1] : group[groupIdx + 1];
          if (!swapWith) return;
          const swapIdx = tasks.indexOf(swapWith);
          tasks.splice(globalIdx, 1);
          tasks.splice(swapIdx, 0, task);
          renderTasks();
        });

        listEl.appendChild(row);
      });

      groupEl.appendChild(listEl);
      taskGroups.appendChild(groupEl);
    });

    updateSubmitState();
  }

  function addTask() {
    const text = taskInput.value.trim();
    if (!text) return;
    tasks.push({ text, category: activeCat });
    taskInput.value = '';
    renderTasks();
    taskInput.focus();
  }

  addTaskBtn.addEventListener('click', addTask);
  taskInput.addEventListener('keydown', (e) => {
    if (e.key === 'Enter') { e.preventDefault(); addTask(); }
  });
  renderCarryovers();
  renderTasks();

  document.getElementById('morning-form').addEventListener('submit', async (e) => {
    e.preventDefault();
    const btn = document.getElementById('morning-btn');
    const err = document.getElementById('morning-error');
    addTask();

    err.hidden = true;
    const selected = selectedCarryovers();
    if (tasks.length + selected.length === 0) {
      err.textContent = 'Add at least one task for today.';
      err.hidden = false;
      taskInput.focus();
      return;
    }

    btn.disabled = true;
    btn.textContent = 'Saving...';

    const intention = document.getElementById('intention').value;
    const notes     = document.getElementById('notes').value;
    const priorities = [
      ...selected.map(({ text, category, carried_from }) => ({ text, category, carried_from, done: false })),
      ...tasks.map(({ text, category }) => ({ text, category, done: false })),
    ];

    try {
      await invoke('submit_plan', { intention, priorities, notes });
      await invoke('close_app');
    } catch (err2) {
      err.textContent = err2;
      err.hidden = false;
      btn.disabled = false;
      btn.textContent = 'Start My Day';
    }
  });
}

// ── Evening lock ──────────────────────────────────────────────────────────────

function startEveningLock(plan) {
  startClock('clock-eve', 'date-eve');
  document.getElementById('review-notes').focus();

  document.getElementById('evening-intention').textContent = plan.intention || "Review today's priorities.";

  const container = document.getElementById('evening-priorities');

  const grouped = CATS.map(({ id, label }) => ({
    id, label,
    items: plan.priorities.map((p, i) => ({ ...p, _i: i })).filter((p) => (p.category || 'personal') === id),
  })).filter((g) => g.items.length > 0);

  grouped.forEach(({ id, label, items }) => {
    const groupHeader = document.createElement('div');
    groupHeader.className = 'cat-group-label';
    groupHeader.dataset.cat = id;
    groupHeader.textContent = label;
    container.appendChild(groupHeader);

    items.forEach(({ _i: i, text, done }) => {
      const row = document.createElement('label');
      row.className = 'review-priority' + (done ? ' done' : '');
      row.innerHTML = `
        <input type="checkbox" ${done ? 'checked' : ''} />
        <span class="review-priority-text">${escHtml(text)}</span>`;

      row.querySelector('input').addEventListener('change', async (e) => {
        try {
          await invoke('toggle_priority', { date: plan.date, index: i });
          row.classList.toggle('done', e.target.checked);
        } catch (err) {
          e.target.checked = !e.target.checked;
          console.error(err);
        }
      });

      container.appendChild(row);
    });
  });

  document.getElementById('evening-form').addEventListener('submit', async (e) => {
    e.preventDefault();
    const btn = document.getElementById('evening-btn');
    const err = document.getElementById('evening-error');
    const reviewNotes = document.getElementById('review-notes').value;

    err.hidden = true;
    btn.disabled = true;
    btn.textContent = 'Saving...';

    try {
      await invoke('submit_review', { reviewNotes });
      showView('dashboard');
      loadDashboard();
    } catch (e) {
      err.textContent = e;
      err.hidden = false;
      btn.disabled = false;
      btn.textContent = 'End My Day';
    }
  });
}

// ── Today sticky ──────────────────────────────────────────────────────────────

function startSticky() {
  document.getElementById('sticky-close-btn').onclick = () => invoke('close_app');
  document.getElementById('sticky-refresh-btn').onclick = () => loadSticky();
  document.getElementById('sticky-dashboard-btn').onclick = async () => {
    await invoke('configure_dashboard_window');
    showView('dashboard');
    loadDashboard();
  };

  document.addEventListener('keydown', (e) => {
    if (e.key === 'Escape') invoke('close_app');
  });

  loadSticky();
}

async function loadSticky() {
  const plan = await invoke('get_today_plan');
  const progress = document.getElementById('sticky-progress');
  const list = document.getElementById('sticky-list');

  if (!plan) {
    progress.textContent = 'No plan set';
    list.innerHTML = `
      <div class="sticky-empty">
        <p>No plan for today yet.</p>
        <button id="sticky-plan-btn" type="button">Plan today</button>
      </div>`;
    document.getElementById('sticky-plan-btn').onclick = async () => {
      await invoke('configure_dashboard_window');
      showView('morning-lock');
      await startMorningLock();
    };
    return;
  }

  const done = plan.priorities.filter((task) => task.done).length;
  const total = plan.priorities.length;
  progress.textContent = `${done}/${total} done`;

  const grouped = CATS.map(({ id, label }) => ({
    id, label,
    items: plan.priorities
      .map((p, i) => ({ ...p, _i: i }))
      .filter((p) => (p.category || 'personal') === id),
  })).filter((g) => g.items.length > 0);

  list.innerHTML = grouped.map(({ id, label, items }) => `
    <div class="sticky-group">
      <div class="cat-group-label" data-cat="${id}">${label}</div>
      ${items.map((task) => `
        <label class="sticky-task ${task.done ? 'done' : ''}" data-idx="${task._i}">
          <input type="checkbox" ${task.done ? 'checked' : ''} />
          <span>${escHtml(task.text)}</span>
        </label>`).join('')}
    </div>
  `).join('');

  list.querySelectorAll('.sticky-task').forEach((row) => {
    const index = Number(row.dataset.idx);
    row.querySelector('input').addEventListener('change', async (e) => {
      e.target.disabled = true;
      try {
        await invoke('toggle_priority', { date: plan.date, index });
        await loadSticky();
      } catch (err) {
        e.target.checked = !e.target.checked;
        e.target.disabled = false;
        console.error(err);
      }
    });
  });
}

// ── Dashboard ─────────────────────────────────────────────────────────────────

let _dashboardBooted = false;

async function loadDashboard() {
  const plans = await invoke('get_all_plans');
  const today = new Date().toISOString().slice(0, 10);
  const todayPlan = plans.find((p) => p.date === today);
  const past = plans.filter((p) => p.date !== today);

  // One-time wiring — guards against listener stacking if called more than once
  if (!_dashboardBooted) {
    _dashboardBooted = true;
    initAiButton();
    initSettings();
    document.getElementById('close-btn').onclick = () => invoke('close_app');
    document.addEventListener('keydown', (e) => {
      if (e.key === 'Escape' && document.getElementById('settings-overlay').hidden) invoke('close_app');
    });
  }

  // Week stats (reuse already-fetched plans — no second round-trip)
  renderWeekStats(plans);

  // Today card
  const todayCard = document.getElementById('today-card');
  if (todayPlan) {
    todayCard.innerHTML = renderCardBody(todayPlan, false);
    wireCheckboxes(todayCard, todayPlan);
  } else {
    todayCard.innerHTML = `<p style="color:var(--text-muted);font-size:.9rem">No plan set for today yet.</p>`;
  }

  // Past plans
  const historySection = document.getElementById('history-section');
  const historyList = document.getElementById('history-list');
  if (past.length > 0) {
    historySection.hidden = false;
    historyList.innerHTML = past.map((p) => renderHistoryCard(p)).join('');
    historyList.querySelectorAll('.history-card').forEach((card, i) => {
      card.querySelector('.card-header').addEventListener('click', () => card.classList.toggle('collapsed'));
      wireCheckboxes(card, past[i]);
    });
  }
}

// ── Week stats ────────────────────────────────────────────────────────────────

function renderWeekStats(plans) {
  const today = new Date();
  const monday = new Date(today);
  monday.setDate(today.getDate() - ((today.getDay() + 6) % 7));

  const weekPlans = plans.filter((p) => {
    const d = new Date(p.date + 'T00:00:00');
    return d >= monday && d <= today;
  });

  const daysPlanned = weekPlans.length;
  const totalTasks = weekPlans.reduce((s, p) => s + p.priorities.length, 0);
  const doneTasks = weekPlans.reduce((s, p) => s + p.priorities.filter((t) => t.done).length, 0);
  const pct = totalTasks > 0 ? Math.round((doneTasks / totalTasks) * 100) : 0;

  document.getElementById('week-stats').innerHTML = `
    <div class="stat"><span class="stat-value">${daysPlanned}</span><span class="stat-label">Days planned</span></div>
    <div class="stat"><span class="stat-value">${doneTasks}/${totalTasks}</span><span class="stat-label">Tasks done</span></div>
    <div class="stat"><span class="stat-value">${pct}%</span><span class="stat-label">Completion</span></div>`;
}

// ── AI Review ─────────────────────────────────────────────────────────────────

function initAiButton() {
  const btn = document.getElementById('ai-btn');
  const card = document.getElementById('ai-card');
  let inflight = false;

  btn.addEventListener('click', async () => {
    if (inflight) return;
    inflight = true;
    btn.disabled = true;
    btn.classList.add('loading');
    btn.textContent = '✦ Thinking…';
    card.hidden = false;
    card.className = 'ai-card loading';
    card.textContent = 'Analysing your week…';

    try {
      const review = await invoke('get_ai_review');
      card.className = 'ai-card';
      card.innerHTML = marked.parse(review);
      btn.textContent = '✦ Refresh';
    } catch (err) {
      card.className = 'ai-card';
      card.innerHTML = `<span class="ai-error">${escHtml(String(err))}</span>`;
      btn.textContent = '✦ AI Review';
    } finally {
      inflight = false;
      btn.disabled = false;
      btn.classList.remove('loading');
    }
  });
}

// ── Settings ──────────────────────────────────────────────────────────────────

async function initSettings() {
  const overlay  = document.getElementById('settings-overlay');
  const openBtn  = document.getElementById('settings-btn');
  const closeBtn = document.getElementById('settings-close');
  const saveBtn  = document.getElementById('save-settings-btn');
  const feedback = document.getElementById('settings-feedback');
  const toggleKey = document.getElementById('toggle-key');
  const apiKeyInput = document.getElementById('api-key');

  // Load current config + timer install check
  const [cfg, timersOk] = await Promise.all([invoke('get_config'), invoke('check_timers')]);
  document.getElementById('morning-time').value = cfg.morning_time || '05:00';
  document.getElementById('evening-time').value = cfg.evening_time || '18:00';
  apiKeyInput.value = cfg.claude_api_key || '';

  if (!timersOk) {
    const hint = document.createElement('p');
    hint.className = 'settings-hint install-hint';
    hint.textContent = 'Timers not installed — locks won\'t fire automatically. Run install.sh to enable scheduling.';
    document.getElementById('settings-btn').closest('.dash-topbar')
      .insertAdjacentElement('afterend', hint);
  }

  // Show/hide key
  toggleKey.addEventListener('click', () => {
    apiKeyInput.type = apiKeyInput.type === 'password' ? 'text' : 'password';
  });

  // Open / close
  openBtn.addEventListener('click', () => { overlay.hidden = false; });
  closeBtn.addEventListener('click', () => { overlay.hidden = true; feedback.hidden = true; });
  overlay.addEventListener('click', (e) => { if (e.target === overlay) overlay.hidden = true; });

  // Save
  saveBtn.addEventListener('click', async () => {
    saveBtn.disabled = true;
    feedback.hidden = true;

    const morningTime = document.getElementById('morning-time').value;
    const eveningTime = document.getElementById('evening-time').value;
    const claudeApiKey = apiKeyInput.value;

    try {
      await invoke('save_settings', { morningTime, eveningTime, claudeApiKey });
      feedback.textContent = 'Saved. Timers updated.';
      feedback.className = 'settings-feedback ok';
      feedback.hidden = false;
      setTimeout(() => { feedback.hidden = true; }, 3000);
    } catch (err) {
      feedback.textContent = String(err);
      feedback.className = 'settings-feedback err';
      feedback.hidden = false;
    } finally {
      saveBtn.disabled = false;
    }
  });
}

function renderCardBody(plan, collapsible) {
  // Group priorities by category, preserving CATS order
  const grouped = CATS.map(({ id, label }) => ({
    id, label,
    items: plan.priorities
      .map((p, i) => ({ ...p, _i: i }))
      .filter((p) => (p.category || 'personal') === id),
  })).filter((g) => g.items.length > 0);

  const prioritiesHtml = grouped.map(({ id, label, items }) => `
    <div class="cat-group-label" data-cat="${id}">${label}</div>
    ${items.map((p) => `
      <div class="card-priority ${p.done ? 'done' : ''}" data-idx="${p._i}">
        <input type="checkbox" ${p.done ? 'checked' : ''} />
        <span class="card-priority-text">${escHtml(p.text)}</span>
        ${p.carried_from ? `<span class="carryover-chip">from ${formatShortDate(p.carried_from)}</span>` : ''}
      </div>`).join('')}
  `).join('');

  const reviewed = plan.reviewed_at
    ? `<div class="card-reviewed">✓ Reviewed</div>` : '';
  const notes = plan.notes
    ? `<div class="card-notes">${escHtml(plan.notes)}</div>` : '';
  const reviewNotes = plan.review_notes
    ? `<div class="card-notes" style="border-top-style:dashed;color:var(--text-muted)">${escHtml(plan.review_notes)}</div>` : '';

  return `
    <div class="card-header">
      <div>
        <div class="card-date">${formatDate(plan.date)}</div>
        <div class="card-intention">${escHtml(plan.intention || 'Today\'s priorities')}</div>
      </div>
      ${collapsible ? '<span class="expand-icon">▼</span>' : reviewed}
    </div>
    <div class="card-priorities">${prioritiesHtml}</div>
    ${notes}
    ${reviewNotes}`;
}

function wireCheckboxes(container, plan) {
  container.querySelectorAll('.card-priority').forEach((row) => {
    const i = Number(row.dataset.idx);
    row.querySelector('input[type="checkbox"]').addEventListener('change', async (e) => {
      try {
        await invoke('toggle_priority', { date: plan.date, index: i });
        row.classList.toggle('done', e.target.checked);
      } catch (err) {
        e.target.checked = !e.target.checked;
        console.error(err);
      }
    });
  });
}

function renderHistoryCard(plan) {
  return `<div class="plan-card history-card collapsed">${renderCardBody(plan, true)}</div>`;
}

function formatDate(iso) {
  return new Date(iso + 'T00:00:00').toLocaleDateString('en-US', {
    weekday: 'long', year: 'numeric', month: 'long', day: 'numeric',
  });
}

function formatShortDate(iso) {
  return new Date(iso + 'T00:00:00').toLocaleDateString('en-US', {
    weekday: 'short',
  });
}

function escHtml(str) {
  return str.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
}

// ── Boot ──────────────────────────────────────────────────────────────────────

try {
  const { mode, today_plan: todayPlan } = await invoke('get_app_state');

  if (mode === 'sticky') {
    await invoke('configure_sticky_window');
    showView('sticky');
    startSticky();
  } else if (mode === 'morning' && !todayPlan) {
    showView('morning-lock');
    await startMorningLock();
  } else if (mode === 'evening' && todayPlan && !todayPlan.reviewed_at) {
    showView('evening-lock');
    startEveningLock(todayPlan);
  } else {
    showView('dashboard');
    loadDashboard();
  }
} catch (err) {
  document.body.innerHTML = `<div style="padding:2rem;color:#f87171;font-family:monospace">
    <strong>Failed to start:</strong><br>${escHtml(String(err))}
  </div>`;
}
