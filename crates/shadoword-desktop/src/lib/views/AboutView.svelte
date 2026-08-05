<script lang="ts">
	import { CodeXml, ExternalLink, FileJson, FolderOpen, ShieldCheck } from '@lucide/svelte';
	import { Badge } from '$lib/components/ui/badge';
	import { Button } from '$lib/components/ui/button';
	import BrandMark from '$lib/components/BrandMark.svelte';
	import SurfaceHeader from '$lib/components/SurfaceHeader.svelte';
</script>

<div class="about-view">
	<SurfaceHeader
		kicker="About"
		title="Local by default, connected by choice."
		description="Shadoword is a Linux-first speech-to-text workspace for local, self-hosted, or direct provider transcription."
	/>

	<section class="identity-block">
		<div class="brand-lockup">
			<BrandMark />
			<p>Run locally, connect to your own API, or opt into OpenRouter per recording.</p>
		</div>
		<div class="version-block">
			<span>Linux desktop</span><strong>0.10.4</strong><Badge variant="outline">Production</Badge>
		</div>
	</section>

	<div class="about-grid">
		<section>
			<header>
				<ShieldCheck size={16} />
				<h2 class="display-legend">Product principles</h2>
			</header>
			<ul>
				<li>
					<span class="numeral-outline">01</span>
					<div>
						<strong>User-controlled inference</strong>
						<p>Run locally, on infrastructure you operate, or through OpenRouter when chosen.</p>
					</div>
				</li>
				<li>
					<span class="numeral-outline">02</span>
					<div>
						<strong>Desktop-native flow</strong>
						<p>Capture with a global shortcut and deliver text to the active application.</p>
					</div>
				</li>
				<li>
					<span class="numeral-outline">03</span>
					<div>
						<strong>Visible system state</strong>
						<p>Know which microphone, model, backend, and output path are active.</p>
					</div>
				</li>
			</ul>
		</section>

		<section>
			<header>
				<FolderOpen size={16} />
				<h2 class="display-legend">Storage</h2>
			</header>
			<div class="path-row">
				<FileJson size={15} />
				<div><span>Desktop configuration</span><code>~/.config/shadoword/desktop.json</code></div>
			</div>
			<div class="path-row">
				<FolderOpen size={15} />
				<div><span>Local models</span><code>~/.local/share/shadoword/models</code></div>
			</div>
		</section>
	</div>

	<section class="technology-strip">
		<div>
			<span>Desktop</span><strong>Tauri 2</strong>
			<p>Small native shell and Linux integration.</p>
		</div>
		<div>
			<span>Interface</span><strong>SvelteKit</strong>
			<p>Reactive UI compiled into static assets.</p>
		</div>
		<div>
			<span>System</span><strong>Rust</strong>
			<p>Audio, VAD, inference, API, and delivery.</p>
		</div>
		<div>
			<span>Components</span><strong>shadcn-svelte</strong>
			<p>Accessible primitives under Shadoword styling.</p>
		</div>
	</section>

	<footer>
		<span>MIT licensed · built in the open</span>
		<div>
			<Button variant="outline" size="sm"><CodeXml size={14} />Source</Button>
			<Button variant="ghost" size="sm">Licenses<ExternalLink size={13} /></Button>
		</div>
	</footer>
</div>

<style>
	.about-view {
		display: grid;
		gap: 1.25rem;
	}
	.identity-block {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 2rem;
		min-height: 8rem;
		border: 1px solid var(--line);
		padding: 1.5rem;
		background: var(--surface-1);
	}
	.brand-lockup p {
		max-width: 38rem;
		margin: 0.6rem 0 0;
		color: var(--ink-dim);
		font-size: 0.8125rem;
		line-height: 1.5;
	}
	.version-block {
		display: grid;
		justify-items: end;
		gap: 0.25rem;
	}
	.version-block > span,
	section header h2,
	.technology-strip span {
		color: var(--ink-muted);
		font-size: 0.6875rem;
		font-weight: 680;
		letter-spacing: 0.08em;
		text-transform: uppercase;
	}
	.version-block strong {
		color: var(--ink);
		font-family: var(--font-mono);
		font-size: 1.25rem;
		font-weight: 550;
	}
	.about-grid {
		display: grid;
		grid-template-columns: 1.1fr 0.9fr;
		gap: 1rem;
	}
	.about-grid > section {
		border: 1px solid var(--line);
		background: var(--surface-1);
	}
	section header {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		border-bottom: 1px solid var(--line);
		padding: 0.8rem 1rem;
		color: var(--ink-dim);
	}
	section header h2 {
		margin: 0;
	}
	ul {
		margin: 0;
		padding: 0;
		list-style: none;
	}
	li {
		display: grid;
		grid-template-columns: 2.1rem 1fr;
		gap: 0.85rem;
		padding: 0.9rem 1rem;
	}
	li + li,
	.path-row + .path-row {
		border-top: 1px solid var(--line);
	}
	/* The index is a stroked display numeral, which is how this world numbers things
	   and the only treatment in the app that lets a large glyph carry no weight at
	   all: hollow, so it orders the list without competing with the principle it is
	   pointing at. It was 11px blue mono, which ordered nothing and read as a link. */
	li > span {
		align-self: start;
		padding-top: 0.05rem;
	}
	li strong,
	.path-row span {
		color: var(--ink);
		font-size: 0.75rem;
		font-weight: 570;
	}
	li p {
		margin: 0.25rem 0 0;
		color: var(--ink-muted);
		font-size: 0.6875rem;
		line-height: 1.45;
	}
	.path-row {
		display: grid;
		grid-template-columns: 1.25rem 1fr;
		gap: 0.65rem;
		align-items: start;
		padding: 1rem;
		color: var(--ink-muted);
	}
	.path-row > div {
		display: grid;
		gap: 0.4rem;
		min-width: 0;
	}
	.path-row code {
		overflow: hidden;
		color: var(--ink-dim);
		font-size: 0.6875rem;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.technology-strip {
		display: grid;
		grid-template-columns: repeat(4, 1fr);
		border: 1px solid var(--line);
		background: var(--surface-1);
	}
	.technology-strip > div {
		padding: 1rem;
	}
	.technology-strip > div + div {
		border-left: 1px solid var(--line);
	}
	.technology-strip strong {
		display: block;
		margin-top: 0.4rem;
		color: var(--ink);
		font-size: 0.8125rem;
		font-weight: 600;
	}
	.technology-strip p {
		margin: 0.35rem 0 0;
		color: var(--ink-muted);
		font-size: 0.6875rem;
		line-height: 1.4;
	}
	.about-view > footer {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 1rem;
		border-top: 1px solid var(--line);
		padding-top: 1rem;
	}
	.about-view > footer > span {
		color: var(--ink-muted);
		font-size: 0.6875rem;
	}
	.about-view > footer > div {
		display: flex;
		gap: 0.35rem;
	}
	@media (max-width: 850px) {
		.about-grid {
			grid-template-columns: 1fr;
		}
		.technology-strip {
			grid-template-columns: 1fr 1fr;
		}
		.technology-strip > div:nth-child(3) {
			border-top: 1px solid var(--line);
			border-left: 0;
		}
		.technology-strip > div:nth-child(4) {
			border-top: 1px solid var(--line);
		}
	}
	@media (max-width: 620px) {
		.identity-block {
			align-items: flex-start;
			flex-direction: column;
		}
		.version-block {
			justify-items: start;
		}
	}
</style>
