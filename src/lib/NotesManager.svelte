<script lang="ts">
  import { onMount } from "svelte";
  import { webviewWindow } from "@tauri-apps/api";
  import { invoke } from "@tauri-apps/api/core";

  type NoteStatus = "open" | "closed" | "archived";

  type NoteListItem = {
    id: string;
    status: NoteStatus;
    created_at: string;
    updated_at: string;
    closed_at?: string;
    archived_at?: string;
    color: string;
    preview: string;
  };

  const appWindow = webviewWindow.getCurrentWebviewWindow();

  let notes = $state<NoteListItem[]>([]);
  let loading = $state(false);
  let error = $state("");
  let activeFilter = $state<"all" | NoteStatus>("all");
  let busyNoteId = $state("");

  const dateFormatter = new Intl.DateTimeFormat(undefined, {
    year: "numeric",
    month: "short",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  });

  async function refreshNotes() {
    loading = true;
    try {
      const data = await invoke<NoteListItem[]>("list_saved_notes");
      notes = data;
      error = "";
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  function formatDate(value?: string) {
    if (!value) return "-";

    const date = new Date(value);
    if (Number.isNaN(date.getTime())) {
      return value;
    }

    return dateFormatter.format(date);
  }

  function filteredNotes() {
    if (activeFilter === "all") {
      return notes;
    }

    return notes.filter((note) => note.status === activeFilter);
  }

  function statusLabel(status: NoteStatus) {
    return status.charAt(0).toUpperCase() + status.slice(1);
  }

  async function runNoteAction(command: string, noteId: string) {
    busyNoteId = noteId;
    error = "";

    try {
      await invoke(command, { noteId });
      await refreshNotes();
    } catch (e) {
      error = String(e);
    } finally {
      busyNoteId = "";
    }
  }

  async function openNotesFolder() {
    try {
      await invoke("open_notes_folder");
    } catch (e) {
      error = String(e);
    }
  }

  function noteCount(status: NoteStatus) {
    return notes.filter((note) => note.status === status).length;
  }

  onMount(() => {
    void refreshNotes();

    const unlisteners: Array<() => void> = [];

    (async () => {
      unlisteners.push(await appWindow.listen("notes_changed", refreshNotes));
      unlisteners.push(await appWindow.listen("tauri://focus", refreshNotes));
    })();

    return () => {
      for (const unlisten of unlisteners) {
        unlisten();
      }
    };
  });
</script>

<div class="manager-root">
  <header class="manager-header" data-tauri-drag-region>
    <div>
      <h1>Notes Manager</h1>
      <p>Restore closed notes, archive old ones, and delete permanently.</p>
    </div>
    <button class="open-folder" onclick={openNotesFolder}>Open Notes Folder</button>
  </header>

  <div class="status-row">
    <button class:active={activeFilter === "all"} onclick={() => (activeFilter = "all")}>All ({notes.length})</button>
    <button class:active={activeFilter === "open"} onclick={() => (activeFilter = "open")}>Open ({noteCount("open")})</button>
    <button class:active={activeFilter === "closed"} onclick={() => (activeFilter = "closed")}>Closed ({noteCount("closed")})</button>
    <button class:active={activeFilter === "archived"} onclick={() => (activeFilter = "archived")}>Archived ({noteCount("archived")})</button>
  </div>

  {#if error}
    <p class="error">{error}</p>
  {/if}

  {#if loading}
    <p class="status-message">Loading notes...</p>
  {:else if filteredNotes().length === 0}
    <p class="status-message">No notes in this section.</p>
  {:else}
    <div class="list-wrap">
      {#each filteredNotes() as note}
        <article class="note-row">
          <div class="note-color" style:background={note.color}></div>
          <div class="note-content">
            <div class="note-meta">
              <span class="note-id">{note.id}</span>
              <span class={`badge ${note.status}`}>{statusLabel(note.status)}</span>
            </div>
            <p class="preview">{note.preview || "(empty note)"}</p>
            <p class="timestamps">
              Updated: {formatDate(note.updated_at)} | Created: {formatDate(note.created_at)}
            </p>
          </div>
          <div class="actions">
            <button
              class="primary"
              onclick={() => runNoteAction("restore_note", note.id)}
              disabled={busyNoteId === note.id}
            >
              {note.status === "open" ? "Focus" : "Restore"}
            </button>
            {#if note.status !== "archived"}
              <button
                onclick={() => runNoteAction("archive_note", note.id)}
                disabled={busyNoteId === note.id}
              >
                Archive
              </button>
            {/if}
            <button
              class="danger"
              onclick={() => {
                if (confirm("Delete this note permanently?")) {
                  void runNoteAction("delete_note", note.id);
                }
              }}
              disabled={busyNoteId === note.id}
            >
              Delete
            </button>
          </div>
        </article>
      {/each}
    </div>
  {/if}
</div>

<style>
  .manager-root {
    display: flex;
    flex-direction: column;
    height: 100%;
    width: 100%;
    background: linear-gradient(160deg, #f8f4de 0%, #f3f6fb 45%, #eef2e8 100%);
    border-radius: 10px;
    overflow: hidden;
    color: #27313a;
  }

  .manager-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    padding: 18px 20px 14px;
    border-bottom: 1px solid rgba(39, 49, 58, 0.14);
    user-select: none;
  }

  h1 {
    margin: 0;
    font-size: 20px;
    font-weight: 700;
  }

  .manager-header p {
    margin: 4px 0 0;
    font-size: 12px;
    opacity: 0.8;
  }

  .open-folder {
    border: 0;
    border-radius: 7px;
    padding: 8px 12px;
    background: #1f5c89;
    color: #fff;
    font-size: 12px;
    cursor: pointer;
  }

  .status-row {
    display: flex;
    gap: 8px;
    padding: 12px 16px;
    border-bottom: 1px solid rgba(39, 49, 58, 0.1);
    overflow-x: auto;
  }

  .status-row button {
    border: 1px solid rgba(39, 49, 58, 0.2);
    border-radius: 999px;
    padding: 6px 10px;
    background: rgba(255, 255, 255, 0.65);
    color: #27313a;
    font-size: 12px;
    cursor: pointer;
    white-space: nowrap;
  }

  .status-row button.active {
    background: #27313a;
    color: #fff;
    border-color: #27313a;
  }

  .error {
    margin: 12px 16px 0;
    color: #9f2121;
    font-size: 12px;
  }

  .status-message {
    margin: 20px 16px;
    font-size: 13px;
    color: rgba(39, 49, 58, 0.7);
  }

  .list-wrap {
    padding: 12px 12px 16px;
    overflow: auto;
    display: grid;
    gap: 10px;
  }

  .note-row {
    display: grid;
    grid-template-columns: 6px 1fr auto;
    gap: 10px;
    padding: 12px;
    background: rgba(255, 255, 255, 0.78);
    border: 1px solid rgba(39, 49, 58, 0.12);
    border-radius: 10px;
    align-items: center;
  }

  .note-color {
    align-self: stretch;
    border-radius: 999px;
  }

  .note-content {
    min-width: 0;
  }

  .note-meta {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .note-id {
    font-family: ui-monospace, SFMono-Regular, SF Mono, Menlo, monospace;
    font-size: 11px;
    color: rgba(39, 49, 58, 0.8);
  }

  .badge {
    font-size: 10px;
    padding: 3px 7px;
    border-radius: 999px;
    border: 1px solid transparent;
  }

  .badge.open {
    background: rgba(24, 129, 62, 0.12);
    border-color: rgba(24, 129, 62, 0.35);
  }

  .badge.closed {
    background: rgba(176, 124, 14, 0.12);
    border-color: rgba(176, 124, 14, 0.35);
  }

  .badge.archived {
    background: rgba(74, 84, 98, 0.14);
    border-color: rgba(74, 84, 98, 0.35);
  }

  .preview {
    margin: 7px 0 5px;
    font-size: 13px;
    line-height: 1.3;
    color: rgba(39, 49, 58, 0.95);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .timestamps {
    margin: 0;
    font-size: 11px;
    color: rgba(39, 49, 58, 0.66);
  }

  .actions {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
    justify-content: flex-end;
  }

  .actions button {
    border: 1px solid rgba(39, 49, 58, 0.2);
    border-radius: 7px;
    padding: 6px 10px;
    background: #fff;
    font-size: 12px;
    cursor: pointer;
  }

  .actions button.primary {
    border-color: #1f5c89;
    color: #1f5c89;
  }

  .actions button.danger {
    border-color: rgba(160, 36, 36, 0.4);
    color: #9f2121;
  }

  .actions button:disabled {
    opacity: 0.6;
    cursor: default;
  }

  @media (max-width: 820px) {
    .note-row {
      grid-template-columns: 6px 1fr;
    }

    .actions {
      grid-column: 2;
      justify-content: flex-start;
    }
  }
</style>
