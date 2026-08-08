<script lang="ts">
	import { CodeXml, ExternalLink, FileJson, FolderOpen, ShieldCheck } from '@lucide/svelte';
	import { Badge } from '$lib/components/ui/badge';
	import { Button } from '$lib/components/ui/button';
	import { BrandMark } from '$lib/components/ui/brand-mark';
	import { SurfaceHeader } from '$lib/components/ui/surface-header';
</script>

<svelte:head>
	<title>About · Shadoword</title>
</svelte:head>

<div class="grid gap-5">
	<SurfaceHeader
		title="Local by default, connected by choice."
		description="Shadoword is a Linux-first speech-to-text workspace for local, self-hosted, or direct provider transcription."
	/>

	<section
		class="flex min-h-32 items-center justify-between gap-8 border border-line bg-plate p-6 [@media(max-width:620px)]:flex-col [@media(max-width:620px)]:items-start"
	>
		<div>
			<BrandMark />
			<p class="mt-[0.6rem] mb-0 max-w-[38rem] text-[0.8125rem] leading-normal text-ink-dim">
				Run locally, connect to your own API, or opt into OpenRouter per recording.
			</p>
		</div>
		<div class="grid justify-items-end gap-1 [@media(max-width:620px)]:justify-items-start">
			<span class="text-[0.6875rem] font-[680] tracking-[0.08em] text-ink-muted uppercase">
				Linux desktop
			</span>
			<strong class="font-mono text-xl font-[550] text-ink">0.13.0</strong>
			<Badge variant="outline">Production</Badge>
		</div>
	</section>

	<div class="grid grid-cols-[1.1fr_0.9fr] gap-4 [@media(max-width:850px)]:grid-cols-1">
		<section class="border border-line bg-plate">
			<header class="flex items-center gap-2 border-b border-line px-4 py-[0.8rem] text-ink-dim">
				<ShieldCheck size={16} />
				<h2
					class="m-0 font-display text-[0.6875rem] leading-none font-[680] tracking-[0.08em] text-ink-muted uppercase"
				>
					Product principles
				</h2>
			</header>
			<ul class="m-0 list-none p-0">
				<li class="grid grid-cols-[2.1rem_1fr] gap-[0.85rem] px-4 py-[0.9rem]">
					<span
						class="origin-left [transform:scaleX(var(--squeeze-label))] self-start pt-[0.05rem] font-display text-[2rem] leading-none font-normal text-transparent [-webkit-text-stroke:1px_var(--line-strong)]"
						>01</span
					>
					<div>
						<strong class="text-xs font-[570] text-ink">User-controlled inference</strong>
						<p class="mt-1 mb-0 text-[0.6875rem] leading-[1.45] text-ink-muted">
							Run locally, on infrastructure you operate, or through OpenRouter when chosen.
						</p>
					</div>
				</li>
				<li class="grid grid-cols-[2.1rem_1fr] gap-[0.85rem] border-t border-line px-4 py-[0.9rem]">
					<span
						class="origin-left [transform:scaleX(var(--squeeze-label))] self-start pt-[0.05rem] font-display text-[2rem] leading-none font-normal text-transparent [-webkit-text-stroke:1px_var(--line-strong)]"
						>02</span
					>
					<div>
						<strong class="text-xs font-[570] text-ink">Desktop-native flow</strong>
						<p class="mt-1 mb-0 text-[0.6875rem] leading-[1.45] text-ink-muted">
							Capture with a global shortcut and deliver text to the active application.
						</p>
					</div>
				</li>
				<li class="grid grid-cols-[2.1rem_1fr] gap-[0.85rem] border-t border-line px-4 py-[0.9rem]">
					<span
						class="origin-left [transform:scaleX(var(--squeeze-label))] self-start pt-[0.05rem] font-display text-[2rem] leading-none font-normal text-transparent [-webkit-text-stroke:1px_var(--line-strong)]"
						>03</span
					>
					<div>
						<strong class="text-xs font-[570] text-ink">Visible system state</strong>
						<p class="mt-1 mb-0 text-[0.6875rem] leading-[1.45] text-ink-muted">
							Know which microphone, model, backend, and output path are active.
						</p>
					</div>
				</li>
			</ul>
		</section>

		<section class="border border-line bg-plate">
			<header class="flex items-center gap-2 border-b border-line px-4 py-[0.8rem] text-ink-dim">
				<FolderOpen size={16} />
				<h2
					class="m-0 font-display text-[0.6875rem] leading-none font-[680] tracking-[0.08em] text-ink-muted uppercase"
				>
					Storage
				</h2>
			</header>
			<div class="grid grid-cols-[1.25rem_1fr] items-start gap-[0.65rem] p-4 text-ink-muted">
				<FileJson size={15} />
				<div class="grid min-w-0 gap-[0.4rem]">
					<span class="text-xs font-[570] text-ink">Desktop configuration</span>
					<code
						class="overflow-hidden text-[0.6875rem] text-ellipsis whitespace-nowrap text-ink-dim"
					>
						~/.config/shadoword/desktop.json
					</code>
				</div>
			</div>
			<div
				class="grid grid-cols-[1.25rem_1fr] items-start gap-[0.65rem] border-t border-line p-4 text-ink-muted"
			>
				<FolderOpen size={15} />
				<div class="grid min-w-0 gap-[0.4rem]">
					<span class="text-xs font-[570] text-ink">Local models</span>
					<code
						class="overflow-hidden text-[0.6875rem] text-ellipsis whitespace-nowrap text-ink-dim"
					>
						~/.local/share/shadoword/models
					</code>
				</div>
			</div>
		</section>
	</div>

	<section
		class="grid grid-cols-4 border border-line bg-plate [@media(max-width:850px)]:grid-cols-2"
	>
		<div class="p-4">
			<span class="text-[0.6875rem] font-[680] tracking-[0.08em] text-ink-muted uppercase">
				Desktop
			</span>
			<strong class="mt-[0.4rem] block text-[0.8125rem] font-semibold text-ink">Tauri 2</strong>
			<p class="mt-[0.35rem] mb-0 text-[0.6875rem] leading-[1.4] text-ink-muted">
				Small native shell and Linux integration.
			</p>
		</div>
		<div class="border-l border-line p-4">
			<span class="text-[0.6875rem] font-[680] tracking-[0.08em] text-ink-muted uppercase">
				Interface
			</span>
			<strong class="mt-[0.4rem] block text-[0.8125rem] font-semibold text-ink">SvelteKit</strong>
			<p class="mt-[0.35rem] mb-0 text-[0.6875rem] leading-[1.4] text-ink-muted">
				Reactive UI compiled into static assets.
			</p>
		</div>
		<div
			class="border-l border-line p-4 [@media(max-width:850px)]:border-t [@media(max-width:850px)]:border-l-0"
		>
			<span class="text-[0.6875rem] font-[680] tracking-[0.08em] text-ink-muted uppercase">
				System
			</span>
			<strong class="mt-[0.4rem] block text-[0.8125rem] font-semibold text-ink">Rust</strong>
			<p class="mt-[0.35rem] mb-0 text-[0.6875rem] leading-[1.4] text-ink-muted">
				Audio, VAD, inference, API, and delivery.
			</p>
		</div>
		<div class="border-l border-line p-4 [@media(max-width:850px)]:border-t">
			<span class="text-[0.6875rem] font-[680] tracking-[0.08em] text-ink-muted uppercase">
				Components
			</span>
			<strong class="mt-[0.4rem] block text-[0.8125rem] font-semibold text-ink">
				shadcn-svelte
			</strong>
			<p class="mt-[0.35rem] mb-0 text-[0.6875rem] leading-[1.4] text-ink-muted">
				Accessible primitives under Shadoword styling.
			</p>
		</div>
	</section>

	<footer class="flex items-center justify-between gap-4 border-t border-line pt-4">
		<span class="text-[0.6875rem] text-ink-muted">MIT licensed · built in the open</span>
		<div class="flex gap-[0.35rem]">
			<Button
				variant="outline"
				size="sm"
				href="https://github.com/Fractal-Tess/shadoword"
				target="_blank"
				rel="noreferrer"
			>
				<CodeXml size={14} />Source<ExternalLink size={13} />
			</Button>
			<Button
				variant="ghost"
				size="sm"
				href="https://github.com/Fractal-Tess/shadoword/blob/main/LICENSE"
				target="_blank"
				rel="noreferrer"
			>
				License<ExternalLink size={13} />
			</Button>
		</div>
	</footer>
</div>
