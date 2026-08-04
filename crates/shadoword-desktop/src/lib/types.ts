export type PageId = 'transcribe' | 'models' | 'history' | 'settings' | 'about';

export type RuntimeMode = 'local' | 'remote';

export type RuntimeState = 'ready' | 'loading' | 'offline' | 'warning';

export interface ModelRecord {
	id: string;
	name: string;
	description: string;
	size: string;
	installed: boolean;
	selected: boolean;
	recommended?: boolean;
}

export interface HistoryRecord {
	id: string;
	timestamp: string;
	engine: string;
	duration: string;
	latency: string;
	text: string;
	segments: number;
}
