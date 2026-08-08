export type PageId =
	| 'transcribe'
	| 'models'
	| 'history'
	| 'settings'
	| 'capture'
	| 'transcription'
	| 'output'
	| 'application'
	| 'about';

export type RuntimeState = 'ready' | 'loading' | 'offline' | 'warning';

export type ModelRecord = {
	id: string;
	name: string;
	description: string;
	size: string;
	installed: boolean;
	selected: boolean;
	recommended?: boolean;
};

/** History outlives the process now, so the native host owns its shape and the
 *  window renders whatever the host stored rather than a display-shaped copy. */
export type { HistoryEntry as HistoryRecord } from '$lib/bindings';
