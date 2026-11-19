/**
 * Build-time chain identity. Every value comes from a `VITE_*` environment variable and falls
 * back to the Ergo mainnet default, so a build with no variables set is the Ergo explorer.
 */

import type { StatusChainDto } from '$lib/api/types';

export const DEFAULT_BLOCK_SECONDS = 120;

/** A non-empty trimmed string, or the fallback. */
export function parseText(raw: unknown, fallback: string): string {
	if (typeof raw !== 'string') return fallback;
	const text = raw.trim();
	return text === '' ? fallback : text;
}

/** A positive integer, or the fallback for anything else (empty, zero, negative, fractional, junk). */
export function parsePositiveInt(raw: unknown, fallback: number): number {
	const text = typeof raw === 'number' ? String(raw) : raw;
	if (typeof text !== 'string' || !/^\s*\d+\s*$/.test(text)) return fallback;
	const value = Number(text);
	return Number.isSafeInteger(value) && value > 0 ? value : fallback;
}

/** True unless the value is "0" or "false" (any case). Unset or empty keeps the default. */
export function parseFlag(raw: unknown, fallback: boolean): boolean {
	if (typeof raw !== 'string') return fallback;
	const text = raw.trim().toLowerCase();
	if (text === '') return fallback;
	if (text === '0' || text === 'false') return false;
	return true;
}

/** Whole blocks in one day, rounded down; at least 1. */
export function blocksPerDay(blockSeconds: number): number {
	return Math.max(1, Math.floor(86400 / blockSeconds));
}

/** Where the footer source link points when `VITE_SOURCE_URL` is unset. */
export const DEFAULT_SOURCE_URL = 'https://github.com/arkadianet';

/** An absolute http(s) URL, or null for anything else (other schemes, relative paths, junk). */
export function parseHttpUrl(raw: unknown): URL | null {
	if (typeof raw !== 'string') return null;
	const text = raw.trim();
	if (!/^https?:\/\/[^/]/i.test(text)) return null;
	try {
		return new URL(text);
	} catch {
		return null;
	}
}

/** The footer source link: the fallback when unset or invalid, null for the literal "none". */
export function parseSourceUrl(raw: unknown, fallback: string): string | null {
	if (typeof raw !== 'string' || raw.trim() === '') return fallback;
	if (raw.trim().toLowerCase() === 'none') return null;
	return parseHttpUrl(raw)?.href ?? fallback;
}

/** "GitHub" for a github.com link, else the neutral "Source". */
export function sourceLabel(url: string): string {
	const host = parseHttpUrl(url)?.hostname ?? '';
	return host === 'github.com' || host.endsWith('.github.com') ? 'GitHub' : 'Source';
}

export interface NetworkLink {
	label: string;
	url: string;
}

/**
 * `Label=https://url,Label2=https://url2` as a list. Entries without a label, without `=`, with
 * a non-http(s) URL or repeating an earlier label are dropped. URLs cannot contain commas.
 */
export function parseNetworkLinks(raw: unknown): NetworkLink[] {
	if (typeof raw !== 'string') return [];
	const links: NetworkLink[] = [];
	for (const entry of raw.split(',')) {
		const eq = entry.indexOf('=');
		if (eq < 0) continue;
		const label = entry.slice(0, eq).trim();
		const url = parseHttpUrl(entry.slice(eq + 1));
		if (label === '' || url === null || links.some((link) => link.label === label)) continue;
		links.push({ label, url: url.href });
	}
	return links;
}

/** The same route on another explorer: its origin and base path, then this page's path and query. */
export function networkHref(base: string, path: string, search = '', hash = ''): string {
	const url = new URL(base);
	const prefix = url.pathname.replace(/\/+$/, '');
	return `${url.origin}${prefix}${path.startsWith('/') ? path : `/${path}`}${search}${hash}`;
}

/** The entry for this explorer: by label when one is given and matches, else by origin; -1 if none. */
export function currentNetworkIndex(links: NetworkLink[], label: string, origin: string): number {
	const byLabel = label === '' ? -1 : links.findIndex((link) => link.label === label);
	if (byLabel >= 0) return byLabel;
	return links.findIndex((link) => new URL(link.url).origin === origin);
}

const env = import.meta.env;

export const CHAIN_NAME = parseText(env.VITE_CHAIN_NAME, 'Ergo');
export const CHAIN_NETWORK = parseText(env.VITE_CHAIN_NETWORK, 'Ergo mainnet');
export const TICKER = parseText(env.VITE_CHAIN_TICKER, 'ERG');
export const NANO_UNIT = 'nano' + TICKER;
export const BLOCK_SECONDS = parsePositiveInt(env.VITE_BLOCK_SECONDS, DEFAULT_BLOCK_SECONDS);
export const BLOCKS_PER_DAY = blocksPerDay(BLOCK_SECONDS);
export const SITE_NAME = parseText(env.VITE_SITE_NAME, 'Kadia');
export const EXPLORER_TITLE = `${CHAIN_NAME} Explorer`;
export const ERGO_APPS = parseFlag(env.VITE_ERGO_APPS, true);
export const SHOW_ERGO_LOGO = parseFlag(env.VITE_SHOW_ERGO_LOGO, true);
/** The one-character chain mark drawn on transaction, token and header art. Ergo's Sigma unless set. */
export const CHAIN_MARK = parseText(env.VITE_CHAIN_MARK, 'Σ');
/** The header mark shows for Ergo's logo, or for any chain that sets its own mark. */
/** Path of a picture to draw in place of the mark (a file in `static/`, for example `/yolo-logo.png`). Empty: none. */
export const CHAIN_LOGO = parseText(env.VITE_CHAIN_LOGO, '');
export const SHOW_MARK = SHOW_ERGO_LOGO || CHAIN_MARK !== 'Σ' || CHAIN_LOGO !== '';
export const SOURCE_URL = parseSourceUrl(env.VITE_SOURCE_URL, DEFAULT_SOURCE_URL);
export const SOURCE_LABEL = SOURCE_URL === null ? '' : sourceLabel(SOURCE_URL);
export const NETWORK_LINKS = parseNetworkLinks(env.VITE_NETWORK_LINKS);
export const NETWORK_LABEL = parseText(env.VITE_NETWORK_LABEL, '');
/** The header network label becomes a switcher only when there is somewhere else to go. */
export const NETWORK_SWITCH = NETWORK_LINKS.length >= 2;

/**
 * How the backend's `/v1/status` chain report differs from this build's constants, one line
 * per difference. Empty when they agree or when the backend does not report a chain.
 */
export function chainMismatches(
	chain: Pick<StatusChainDto, 'block_seconds' | 'ticker'> | null | undefined,
	blockSeconds: number = BLOCK_SECONDS,
	ticker: string = TICKER
): string[] {
	if (!chain) return [];
	const found: string[] = [];
	if (chain.block_seconds !== blockSeconds)
		found.push(`block_seconds: server ${chain.block_seconds}, build ${blockSeconds}`);
	if (chain.ticker !== ticker) found.push(`ticker: server ${chain.ticker}, build ${ticker}`);
	return found;
}

let chainWarned = false;

/** Log a console warning, once per page load, when the backend and the build disagree. */
export function warnOnChainMismatch(
	chain: Pick<StatusChainDto, 'block_seconds' | 'ticker'> | null | undefined
): void {
	if (chainWarned) return;
	const found = chainMismatches(chain);
	if (found.length === 0) return;
	chainWarned = true;
	console.warn(
		`This build's chain settings differ from the explorer API (${found.join('; ')}). ` +
			'Rebuild the frontend with matching VITE_* variables.'
	);
}
