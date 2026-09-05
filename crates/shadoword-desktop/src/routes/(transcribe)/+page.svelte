<script lang="ts">
	import CaptureStage from './CaptureStage.svelte';
	import TranscriptSurface from './TranscriptSurface.svelte';
	import TranscribeHeader from './TranscribeHeader.svelte';
	import { TranscribeState, setTranscribeContext } from './state.svelte';
	import { useDesktopShell } from '$lib/shell/desktop-shell-context';

	setTranscribeContext(new TranscribeState(useDesktopShell()));
</script>

<svelte:head>
	<title>Transcribe · Shadoword</title>
</svelte:head>

<div
	class="grid h-full min-h-0 flex-1 grid-rows-[auto_minmax(18rem,1fr)_minmax(9.5rem,0.56fr)] gap-[0.85rem] [@media(max-height:720px)]:grid-rows-[auto_minmax(13rem,1fr)_minmax(7.5rem,0.5fr)] [@media(max-height:720px)]:gap-[0.65rem]"
>
	<TranscribeHeader />
	<CaptureStage />
	<TranscriptSurface />
</div>
