import type { HistoryRecord, ModelRecord } from '$lib/types';

export const demoModels: ModelRecord[] = [
	{
		id: 'turbo',
		name: 'Large v3 Turbo',
		description: 'Fast, high-accuracy multilingual transcription for daily use.',
		size: '1.62 GiB',
		installed: true,
		selected: true,
		recommended: true
	},
	{
		id: 'large-v3',
		name: 'Large v3',
		description: 'Highest catalog accuracy when latency and memory are secondary.',
		size: '2.88 GiB',
		installed: true,
		selected: false
	},
	{
		id: 'medium-en',
		name: 'Medium English',
		description: 'English-only model with a smaller memory footprint.',
		size: '1.42 GiB',
		installed: false,
		selected: false
	},
	{
		id: 'small-en',
		name: 'Small English',
		description: 'Compact model for lower-power systems and quick drafts.',
		size: '466 MiB',
		installed: false,
		selected: false
	}
];

const hoursAgo = (hours: number) => new Date(Date.now() - hours * 3_600_000).toISOString();

export const demoHistory: HistoryRecord[] = [
	{
		id: 'history-1',
		recorded_at: hoursAgo(1),
		mode: 'open_router',
		engine: 'Whisper Large v3',
		audio_duration_ms: 8400,
		elapsed_ms: 612,
		segments: 1,
		cost_usd: 0.000125,
		text: 'Keep the API on the workstation and let the laptop remain a lightweight capture client while traveling.'
	},
	{
		id: 'history-2',
		recorded_at: hoursAgo(2),
		mode: 'local',
		engine: 'Large v3 Turbo',
		audio_duration_ms: 4100,
		elapsed_ms: 438,
		segments: 1,
		cost_usd: null,
		text: 'The pause should close the segment without dropping the short word before it.'
	},
	{
		id: 'history-3',
		recorded_at: hoursAgo(26),
		mode: 'remote',
		engine: 'Large v3 Turbo',
		audio_duration_ms: 12700,
		elapsed_ms: 701,
		segments: 4,
		cost_usd: null,
		text: 'Archive the accepted audio beside its response metadata so we can compare the microphone input with the final transcript.'
	}
];
