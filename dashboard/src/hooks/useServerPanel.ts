import { useReducer, useCallback } from "react";
import {
  getServerDocuments,
  uploadServerDocument,
  deleteServerDocument,
} from "@/app/actions/server-panel";
import type { ServerDocument } from "@/types/gateway";

interface PanelState {
  description: string;
  notes: string;
  documents: ServerDocument[];
  loadingDocs: boolean;
  uploading: boolean;
  dragActive: boolean;
  saving: boolean;
  error: string | null;
}

type PanelAction =
  | { type: "INIT"; docs: ServerDocument[]; meta: { description: string; notes: string } }
  | { type: "SET_DESCRIPTION"; value: string }
  | { type: "SET_NOTES"; value: string }
  | { type: "SET_DRAG_ACTIVE"; value: boolean }
  | { type: "SET_UPLOADING"; value: boolean }
  | { type: "SET_SAVING"; value: boolean }
  | { type: "SET_ERROR"; value: string | null }
  | { type: "ADD_DOCUMENTS"; docs: ServerDocument[] }
  | { type: "REMOVE_DOCUMENT"; id: number };

const initialState: PanelState = {
  description: "",
  notes: "",
  documents: [],
  loadingDocs: true,
  uploading: false,
  dragActive: false,
  saving: false,
  error: null,
};

function panelReducer(state: PanelState, action: PanelAction): PanelState {
  switch (action.type) {
    case "INIT":
      return {
        ...state,
        documents: action.docs,
        description: action.meta.description,
        notes: action.meta.notes,
        loadingDocs: false,
      };
    case "SET_DESCRIPTION":
      return { ...state, description: action.value };
    case "SET_NOTES":
      return { ...state, notes: action.value };
    case "SET_DRAG_ACTIVE":
      return { ...state, dragActive: action.value };
    case "SET_UPLOADING":
      return { ...state, uploading: action.value };
    case "SET_SAVING":
      return { ...state, saving: action.value };
    case "SET_ERROR":
      return { ...state, error: action.value };
    case "ADD_DOCUMENTS":
      return {
        ...state,
        documents: [
          ...state.documents.filter((d) => !action.docs.some((n) => n.filename === d.filename)),
          ...action.docs,
        ],
      };
    case "REMOVE_DOCUMENT":
      return { ...state, documents: state.documents.filter((d) => d.id !== action.id) };
    default:
      return state;
  }
}

const ALLOWED_EXTENSIONS = [".txt", ".json", ".yaml", ".yml", ".toml", ".md", ".conf", ".cfg", ".ini", ".log"];
const MAX_FILE_SIZE = 1024 * 1024;

function loadMetaFromStorage(agentId: string): { description: string; notes: string } {
  if (typeof window === "undefined") return { description: "", notes: "" };
  try {
    const stored = localStorage.getItem(`agent_meta_${agentId}`);
    if (stored) {
      const parsed = JSON.parse(stored);
      return { description: parsed.description || "", notes: parsed.notes || "" };
    }
  } catch {}
  return { description: "", notes: "" };
}

export function useServerPanel(agentId: string) {
  const [state, dispatch] = useReducer(panelReducer, initialState);

  const loadData = useCallback(async () => {
    const [docs, meta] = await Promise.all([
      getServerDocuments(agentId),
      Promise.resolve(loadMetaFromStorage(agentId)),
    ]);
    dispatch({ type: "INIT", docs, meta });
  }, [agentId]);

  const saveMeta = useCallback(async () => {
    dispatch({ type: "SET_SAVING", value: true });
    try {
      localStorage.setItem(
        `agent_meta_${agentId}`,
        JSON.stringify({
          description: state.description,
          notes: state.notes,
          updatedAt: new Date().toISOString(),
        })
      );
      await new Promise((r) => setTimeout(r, 200));
    } finally {
      dispatch({ type: "SET_SAVING", value: false });
    }
  }, [agentId, state.description, state.notes]);

  const uploadFiles = useCallback(
    async (files: FileList | null) => {
      if (!files?.length) return;
      dispatch({ type: "SET_UPLOADING", value: true });
      dispatch({ type: "SET_ERROR", value: null });

      const uploaded: ServerDocument[] = [];

      for (const file of Array.from(files)) {
        const ext = "." + (file.name.split(".").pop()?.toLowerCase() || "");

        if (!ALLOWED_EXTENSIONS.includes(ext)) {
          dispatch({ type: "SET_ERROR", value: `Formato no permitido: ${file.name}` });
          continue;
        }

        if (file.size > MAX_FILE_SIZE) {
          dispatch({ type: "SET_ERROR", value: `Archivo muy grande: ${file.name}` });
          continue;
        }

        const content = await file.text();
        const doc = await uploadServerDocument(agentId, {
          filename: file.name,
          file_type: ext,
          file_size: file.size,
          content,
        });

        if (doc) {
          uploaded.push(doc);
        } else {
          dispatch({ type: "SET_ERROR", value: `Error subiendo: ${file.name}` });
        }
      }

      if (uploaded.length > 0) {
        dispatch({ type: "ADD_DOCUMENTS", docs: uploaded });
      }

      dispatch({ type: "SET_UPLOADING", value: false });
    },
    [agentId]
  );

  const removeDocument = useCallback(async (docId: number) => {
    const ok = await deleteServerDocument(docId);
    if (ok) {
      dispatch({ type: "REMOVE_DOCUMENT", id: docId });
    } else {
      dispatch({ type: "SET_ERROR", value: "Error eliminando documento" });
    }
  }, []);

  const handleDragEnter = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    dispatch({ type: "SET_DRAG_ACTIVE", value: true });
  }, []);

  const handleDragLeave = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    dispatch({ type: "SET_DRAG_ACTIVE", value: false });
  }, []);

  const handleDrop = useCallback(
    (e: React.DragEvent) => {
      e.preventDefault();
      e.stopPropagation();
      dispatch({ type: "SET_DRAG_ACTIVE", value: false });
      uploadFiles(e.dataTransfer.files);
    },
    [uploadFiles]
  );

  const setDescription = useCallback((value: string) => {
    dispatch({ type: "SET_DESCRIPTION", value });
  }, []);

  const setNotes = useCallback((value: string) => {
    dispatch({ type: "SET_NOTES", value });
  }, []);

  return {
    ...state,
    loadData,
    saveMeta,
    uploadFiles,
    removeDocument,
    setDescription,
    setNotes,
    handleDragEnter,
    handleDragLeave,
    handleDragOver: handleDragEnter,
    handleDrop,
    allowedExtensions: ALLOWED_EXTENSIONS,
  };
}

export function formatFileSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1048576) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / 1048576).toFixed(1)} MB`;
}
