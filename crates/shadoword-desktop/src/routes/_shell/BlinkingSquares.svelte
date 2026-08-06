<script lang="ts">
	import { cn } from '$lib/utils';
	import type { Snippet } from 'svelte';
	import type { Attachment } from 'svelte/attachments';
	import type { ClassValue } from 'svelte/elements';

	type Direction = 'right' | 'left' | 'top' | 'bottom';

	let {
		class: className,
		fill = false,
		active = true,
		children,
		direction = 'right',
		gridSize = 52,
		squareColor = '#e6202c',
		backgroundColor = '#07090d',
		falloff = 1.6,
		fadeStart = 0.08,
		fadeEnd = 1,
		squareSize = 0.5,
		minBrightness = 0.3,
		twinkleSpeed = 0.45,
		twinkleStrength = 0.35,
		intensity = 0.65,
		opacity = 0.42,
		dpr = 1.25
	}: {
		class?: ClassValue;
		fill?: boolean;
		active?: boolean;
		children?: Snippet;
		direction?: Direction;
		gridSize?: number;
		squareColor?: string;
		backgroundColor?: string;
		falloff?: number;
		fadeStart?: number;
		fadeEnd?: number;
		squareSize?: number;
		minBrightness?: number;
		twinkleSpeed?: number;
		twinkleStrength?: number;
		intensity?: number;
		opacity?: number;
		dpr?: number;
	} = $props();

	const vertexShader = `
		attribute vec2 aPosition;
		varying vec2 vUv;
		void main() {
			vUv = aPosition * 0.5 + 0.5;
			gl_Position = vec4(aPosition, 0.0, 1.0);
		}
	`;

	const fragmentShader = `
		precision highp float;
		varying vec2 vUv;
		uniform vec2 uRes;
		uniform float uTime;
		uniform float uGrid;
		uniform vec2 uDir;
		uniform float uFalloff;
		uniform float uFadeStart;
		uniform float uFadeEnd;
		uniform float uSquareSize;
		uniform float uMinBright;
		uniform float uTwinkleSpeed;
		uniform float uTwinkleStrength;
		uniform float uIntensity;
		uniform float uAlpha;
		uniform vec3 uSquare;
		uniform vec3 uBg;

		float hash21(vec2 p) {
			p = fract(p * vec2(123.34, 456.21));
			p += dot(p, p + 45.32);
			return fract(p.x * p.y);
		}

		void main() {
			float aspect = uRes.x / max(uRes.y, 1.0);
			vec2 cellsXY = vec2(uGrid * aspect, uGrid);
			if (aspect < 1.0) cellsXY = vec2(uGrid, uGrid / max(aspect, 0.0001));
			vec2 gridUv = vUv * cellsXY;
			vec2 cellId = floor(gridUv);
			vec2 cellUv = fract(gridUv) - 0.5;
			vec2 cellCenter = (cellId + 0.5) / cellsXY;
			vec2 centered = cellCenter * 2.0 - 1.0;
			float t = clamp(dot(centered, uDir) * 0.5 + 0.5, 0.0, 1.0);
			float fs = clamp(uFadeStart, 0.0, 0.999);
			float fe = clamp(uFadeEnd, fs + 0.001, 1.0);
			float remap = clamp((t - fs) / (fe - fs), 0.0, 1.0);
			float density = pow(remap, max(uFalloff, 0.0001));
			float gate = hash21(cellId + 11.7);
			float brightnessRandom = hash21(cellId + 47.3);
			float phaseRandom = hash21(cellId + 91.1);
			float lit = step(gate, density);
			float halfSize = clamp(uSquareSize, 0.05, 0.98) * 0.5;
			float inside = step(abs(cellUv.x), halfSize) * step(abs(cellUv.y), halfSize);
			float baseBrightness = mix(clamp(uMinBright, 0.0, 1.0), 1.0, brightnessRandom);
			float phase = phaseRandom * 6.2831853;
			float speed = uTwinkleSpeed * (0.6 + 0.8 * brightnessRandom);
			float pulse = 0.5 + 0.5 * sin(uTime * speed + phase);
			float twinkle = mix(1.0 - uTwinkleStrength, 1.0, pulse);
			float mask = inside * lit * baseBrightness * twinkle * uIntensity;
			vec3 color = mix(uBg, uSquare, clamp(mask, 0.0, 1.0));
			gl_FragColor = vec4(color, uAlpha);
		}
	`;

	function colorChannels(hex: string) {
		const match = /^#?([a-f\d]{2})([a-f\d]{2})([a-f\d]{2})$/i.exec(hex);
		if (!match) return [0, 0, 0] as const;
		return [
			Number.parseInt(match[1], 16) / 255,
			Number.parseInt(match[2], 16) / 255,
			Number.parseInt(match[3], 16) / 255
		] as const;
	}

	function directionVector(value: Direction) {
		if (value === 'left') return [-1, 0] as const;
		if (value === 'top') return [0, 1] as const;
		if (value === 'bottom') return [0, -1] as const;
		return [1, 0] as const;
	}

	const renderSquares: Attachment<HTMLCanvasElement> = (canvas) => {
		const context = canvas.getContext('webgl', {
			alpha: true,
			antialias: false,
			powerPreference: 'high-performance'
		});
		if (!context) return;
		const gl: WebGLRenderingContext = context;
		const reducedMotion = window.matchMedia('(prefers-reduced-motion: reduce)');

		function compile(type: number, source: string) {
			const shader = gl.createShader(type);
			if (!shader) throw new Error('Unable to create the blinking-squares shader.');
			gl.shaderSource(shader, source);
			gl.compileShader(shader);
			if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
				const message = gl.getShaderInfoLog(shader) ?? 'Unknown shader compilation error.';
				gl.deleteShader(shader);
				throw new Error(message);
			}
			return shader;
		}

		const vertex = compile(gl.VERTEX_SHADER, vertexShader);
		const fragment = compile(gl.FRAGMENT_SHADER, fragmentShader);
		const program = gl.createProgram();
		if (!program) throw new Error('Unable to create the blinking-squares program.');
		gl.attachShader(program, vertex);
		gl.attachShader(program, fragment);
		gl.linkProgram(program);
		if (!gl.getProgramParameter(program, gl.LINK_STATUS)) {
			throw new Error(
				gl.getProgramInfoLog(program) ?? 'Unable to link the blinking-squares shader.'
			);
		}
		gl.useProgram(program);

		const buffer = gl.createBuffer();
		if (!buffer) throw new Error('Unable to create the blinking-squares buffer.');
		gl.bindBuffer(gl.ARRAY_BUFFER, buffer);
		gl.bufferData(
			gl.ARRAY_BUFFER,
			new Float32Array([-1, -1, 1, -1, -1, 1, -1, 1, 1, -1, 1, 1]),
			gl.STATIC_DRAW
		);
		const position = gl.getAttribLocation(program, 'aPosition');
		gl.enableVertexAttribArray(position);
		gl.vertexAttribPointer(position, 2, gl.FLOAT, false, 0, 0);

		function uniform(name: string) {
			const location = gl.getUniformLocation(program, name);
			if (location === null) throw new Error(`Missing blinking-squares uniform: ${name}`);
			return location;
		}

		const uniforms = {
			resolution: uniform('uRes'),
			time: uniform('uTime'),
			grid: uniform('uGrid'),
			direction: uniform('uDir'),
			falloff: uniform('uFalloff'),
			fadeStart: uniform('uFadeStart'),
			fadeEnd: uniform('uFadeEnd'),
			squareSize: uniform('uSquareSize'),
			minBrightness: uniform('uMinBright'),
			twinkleSpeed: uniform('uTwinkleSpeed'),
			twinkleStrength: uniform('uTwinkleStrength'),
			intensity: uniform('uIntensity'),
			alpha: uniform('uAlpha'),
			square: uniform('uSquare'),
			background: uniform('uBg')
		};
		let frame: number | null = null;
		let previousFrame = 0;
		let enabled = false;
		let renderConfig = {
			direction: 'right' as Direction,
			gridSize: 52,
			squareColor: '#e6202c',
			backgroundColor: '#07090d',
			falloff: 1.6,
			fadeStart: 0.08,
			fadeEnd: 1,
			squareSize: 0.5,
			minBrightness: 0.3,
			twinkleSpeed: 0.45,
			twinkleStrength: 0.35,
			intensity: 0.65,
			opacity: 0.42,
			dpr: 1.25
		};
		const startedAt = performance.now();

		function resize() {
			const ratio = Math.min(Math.max(renderConfig.dpr, 1), 3, window.devicePixelRatio || 1);
			const nextWidth = Math.max(1, Math.round(canvas.clientWidth * ratio));
			const nextHeight = Math.max(1, Math.round(canvas.clientHeight * ratio));
			if (canvas.width !== nextWidth || canvas.height !== nextHeight) {
				canvas.width = nextWidth;
				canvas.height = nextHeight;
				gl.viewport(0, 0, nextWidth, nextHeight);
			}
		}

		function renderFrame(now: number, animate: boolean) {
			resize();
			const [directionX, directionY] = directionVector(renderConfig.direction);
			const [squareRed, squareGreen, squareBlue] = colorChannels(renderConfig.squareColor);
			const [backgroundRed, backgroundGreen, backgroundBlue] = colorChannels(
				renderConfig.backgroundColor
			);
			gl.uniform2f(uniforms.resolution, canvas.width, canvas.height);
			gl.uniform1f(uniforms.time, animate ? (now - startedAt) / 1000 : 0);
			gl.uniform1f(uniforms.grid, Math.min(Math.max(renderConfig.gridSize, 8), 200));
			gl.uniform2f(uniforms.direction, directionX, directionY);
			gl.uniform1f(uniforms.falloff, renderConfig.falloff);
			gl.uniform1f(uniforms.fadeStart, renderConfig.fadeStart);
			gl.uniform1f(uniforms.fadeEnd, renderConfig.fadeEnd);
			gl.uniform1f(uniforms.squareSize, renderConfig.squareSize);
			gl.uniform1f(uniforms.minBrightness, renderConfig.minBrightness);
			gl.uniform1f(uniforms.twinkleSpeed, renderConfig.twinkleSpeed);
			gl.uniform1f(uniforms.twinkleStrength, animate ? renderConfig.twinkleStrength : 0);
			gl.uniform1f(uniforms.intensity, renderConfig.intensity);
			gl.uniform1f(uniforms.alpha, renderConfig.opacity);
			gl.uniform3f(uniforms.square, squareRed, squareGreen, squareBlue);
			gl.uniform3f(uniforms.background, backgroundRed, backgroundGreen, backgroundBlue);
			gl.drawArrays(gl.TRIANGLES, 0, 6);
		}

		function stopLoop() {
			if (frame !== null) cancelAnimationFrame(frame);
			frame = null;
		}

		function draw(now: number) {
			if (!enabled || document.hidden || reducedMotion.matches) {
				frame = null;
				return;
			}
			if (now - previousFrame >= 1000 / 30) {
				previousFrame = now;
				renderFrame(now, true);
			}
			frame = requestAnimationFrame(draw);
		}

		function syncLoop() {
			stopLoop();
			if (!enabled || document.hidden) return;
			const now = performance.now();
			renderFrame(now, !reducedMotion.matches);
			if (!reducedMotion.matches) frame = requestAnimationFrame(draw);
		}

		function handleVisibilityChange() {
			syncLoop();
		}

		function handleMotionPreference() {
			syncLoop();
		}

		const observer = new ResizeObserver(() => {
			resize();
			if (enabled && !document.hidden) renderFrame(performance.now(), !reducedMotion.matches);
		});
		observer.observe(canvas);
		document.addEventListener('visibilitychange', handleVisibilityChange);
		reducedMotion.addEventListener('change', handleMotionPreference);

		$effect(() => {
			enabled = active;
			renderConfig = {
				direction,
				gridSize,
				squareColor,
				backgroundColor,
				falloff,
				fadeStart,
				fadeEnd,
				squareSize,
				minBrightness,
				twinkleSpeed,
				twinkleStrength,
				intensity,
				opacity,
				dpr
			};
			syncLoop();
			return stopLoop;
		});

		return () => {
			stopLoop();
			observer.disconnect();
			document.removeEventListener('visibilitychange', handleVisibilityChange);
			reducedMotion.removeEventListener('change', handleMotionPreference);
			gl.deleteBuffer(buffer);
			gl.deleteProgram(program);
			gl.deleteShader(vertex);
			gl.deleteShader(fragment);
		};
	};
</script>

<div
	class={cn(
		'size-full overflow-hidden transition-opacity duration-[850ms] ease-[cubic-bezier(0.22,1,0.36,1)] motion-reduce:transition-none',
		fill ? 'absolute inset-0' : 'relative',
		active ? 'opacity-100 will-change-[opacity]' : 'opacity-0',
		className
	)}
>
	<canvas
		class="pointer-events-none absolute inset-0 block size-full"
		{@attach renderSquares}
		aria-hidden="true"
	></canvas>
	{#if children}
		<div class="relative z-1">{@render children()}</div>
	{/if}
</div>
