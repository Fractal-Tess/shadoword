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

export type HistoryRecord = {
	id: string;
	timestamp: string;
	engine: string;
	duration: string;
	latency: string;
	text: string;
	segments: number;
	costUsd?: number;
};
