import { describe, expect, it, vi } from 'vitest';
import {
	BLOCKS_PER_DAY,
	BLOCK_SECONDS,
	CHAIN_NAME,
	CHAIN_NETWORK,
	DEFAULT_SOURCE_URL,
	ERGO_APPS,
	EXPLORER_TITLE,
	NANO_UNIT,
	NETWORK_LABEL,
	NETWORK_LINKS,
	NETWORK_SWITCH,
	SHOW_ERGO_LOGO,
	SITE_NAME,
	SOURCE_LABEL,
	SOURCE_URL,
	TICKER,
	blocksPerDay,
	chainMismatches,
	currentNetworkIndex,
	networkHref,
	parseFlag,
	parseHttpUrl,
	parseNetworkLinks,
	parsePositiveInt,
	parseSourceUrl,
	parseText,
	sourceLabel,
	warnOnChainMismatch
} from '../../src/lib/chain';

describe('chain constants', () => {
	it('default to Ergo mainnet when no variables are set', () => {
		// Skipped when the suite itself is run with chain variables.
		if (import.meta.env.VITE_CHAIN_TICKER) return;
		expect(CHAIN_NAME).toBe('Ergo');
		expect(CHAIN_NETWORK).toBe('Ergo mainnet');
		expect(TICKER).toBe('ERG');
		expect(NANO_UNIT).toBe('nanoERG');
		expect(BLOCK_SECONDS).toBe(120);
		expect(BLOCKS_PER_DAY).toBe(720);
		expect(SITE_NAME).toBe('Kadia');
		expect(EXPLORER_TITLE).toBe('Ergo Explorer');
		expect(ERGO_APPS).toBe(true);
	});

	it('keep the upstream header and footer when no variables are set', () => {
		const env = import.meta.env;
		if (env.VITE_SHOW_ERGO_LOGO || env.VITE_SOURCE_URL || env.VITE_NETWORK_LINKS) return;
		expect(SHOW_ERGO_LOGO).toBe(true);
		expect(SOURCE_URL).toBe('https://github.com/arkadianet');
		expect(SOURCE_LABEL).toBe('GitHub');
		expect(NETWORK_LINKS).toEqual([]);
		expect(NETWORK_LABEL).toBe('');
		expect(NETWORK_SWITCH).toBe(false);
	});
});

describe('parsePositiveInt', () => {
	it('accepts positive integers', () => {
		expect(parsePositiveInt('30', 120)).toBe(30);
		expect(parsePositiveInt(' 45 ', 120)).toBe(45);
	});
	it('falls back on anything invalid', () => {
		for (const raw of [undefined, '', 'abc', '0', '-5', '1.5', '30s', '1e3', null, {}]) {
			expect(parsePositiveInt(raw, 120)).toBe(120);
		}
	});
});

describe('parseFlag', () => {
	it('turns off on 0 or false', () => {
		expect(parseFlag('0', true)).toBe(false);
		expect(parseFlag('false', true)).toBe(false);
		expect(parseFlag('FALSE', true)).toBe(false);
	});
	it('keeps the default when unset or empty', () => {
		expect(parseFlag(undefined, true)).toBe(true);
		expect(parseFlag('', true)).toBe(true);
	});
	it('is on for other values', () => {
		expect(parseFlag('1', false)).toBe(true);
		expect(parseFlag('true', false)).toBe(true);
	});
});

describe('parseText and blocksPerDay', () => {
	it('falls back on empty text', () => {
		expect(parseText(undefined, 'ERG')).toBe('ERG');
		expect(parseText('  ', 'ERG')).toBe('ERG');
		expect(parseText(' YOLO ', 'ERG')).toBe('YOLO');
	});
	it('rounds blocks per day down', () => {
		expect(blocksPerDay(120)).toBe(720);
		expect(blocksPerDay(30)).toBe(2880);
		expect(blocksPerDay(7)).toBe(12342);
		expect(blocksPerDay(200000)).toBe(1);
	});
});

describe('parseHttpUrl', () => {
	it('accepts absolute http and https URLs', () => {
		expect(parseHttpUrl('https://example.org')?.href).toBe('https://example.org/');
		expect(parseHttpUrl(' http://127.0.0.1:5173/x ')?.href).toBe('http://127.0.0.1:5173/x');
		expect(parseHttpUrl('HTTPS://Example.org/a')?.href).toBe('https://example.org/a');
	});
	it('rejects other schemes, relative paths and junk', () => {
		for (const raw of [
			undefined,
			null,
			42,
			'',
			'example.org',
			'/testnet',
			'//example.org',
			'https:example.org',
			'https:///path',
			'javascript:alert(1)',
			'ftp://example.org',
			'data:text/html,hi',
			'http://exa mple.org'
		]) {
			expect(parseHttpUrl(raw)).toBeNull();
		}
	});
});

describe('parseSourceUrl', () => {
	it('keeps the default when unset or empty', () => {
		expect(parseSourceUrl(undefined, DEFAULT_SOURCE_URL)).toBe(DEFAULT_SOURCE_URL);
		expect(parseSourceUrl('', DEFAULT_SOURCE_URL)).toBe(DEFAULT_SOURCE_URL);
		expect(parseSourceUrl('   ', DEFAULT_SOURCE_URL)).toBe(DEFAULT_SOURCE_URL);
	});
	it('hides the link for none, in any case', () => {
		expect(parseSourceUrl('none', DEFAULT_SOURCE_URL)).toBeNull();
		expect(parseSourceUrl(' NONE ', DEFAULT_SOURCE_URL)).toBeNull();
	});
	it('uses an http(s) URL', () => {
		expect(parseSourceUrl('https://git.example.org/yolo/explorer', DEFAULT_SOURCE_URL)).toBe(
			'https://git.example.org/yolo/explorer'
		);
	});
	it('falls back on anything that is not an http(s) URL', () => {
		for (const raw of ['javascript:alert(1)', 'git@github.com:a/b.git', 'nonee', 'off']) {
			expect(parseSourceUrl(raw, DEFAULT_SOURCE_URL)).toBe(DEFAULT_SOURCE_URL);
		}
	});
	it('labels GitHub links as GitHub and others as Source', () => {
		expect(sourceLabel(DEFAULT_SOURCE_URL)).toBe('GitHub');
		expect(sourceLabel('https://www.github.com/x')).toBe('GitHub');
		expect(sourceLabel('https://gitlab.com/x')).toBe('Source');
		expect(sourceLabel('https://notgithub.com/x')).toBe('Source');
	});
});

describe('parseNetworkLinks', () => {
	it('is empty when unset or empty', () => {
		expect(parseNetworkLinks(undefined)).toEqual([]);
		expect(parseNetworkLinks('')).toEqual([]);
		expect(parseNetworkLinks(' , ')).toEqual([]);
	});
	it('parses labels and URLs in order', () => {
		expect(
			parseNetworkLinks('Testnet=https://testnet.example.org, Mainnet = https://example.org/')
		).toEqual([
			{ label: 'Testnet', url: 'https://testnet.example.org/' },
			{ label: 'Mainnet', url: 'https://example.org/' }
		]);
	});
	it('splits on the first = only, so a URL may carry a query', () => {
		expect(parseNetworkLinks('Dev=http://localhost:5173/?a=b')).toEqual([
			{ label: 'Dev', url: 'http://localhost:5173/?a=b' }
		]);
	});
	it('drops malformed entries and keeps the rest', () => {
		expect(
			parseNetworkLinks(
				'Testnet,=https://x.org,Blank=,Bad=not a url,Js=javascript:alert(1),Ftp=ftp://x.org,Rel=/main,Mainnet=https://example.org'
			)
		).toEqual([{ label: 'Mainnet', url: 'https://example.org/' }]);
	});
	it('keeps the first of a repeated label', () => {
		expect(parseNetworkLinks('Main=https://a.org,Main=https://b.org')).toEqual([
			{ label: 'Main', url: 'https://a.org/' }
		]);
	});
});

describe('networkHref', () => {
	it('keeps the path, query and fragment on the other origin', () => {
		expect(networkHref('https://example.org/', '/tx/abc', '?page=2', '#out')).toBe(
			'https://example.org/tx/abc?page=2#out'
		);
		expect(networkHref('https://example.org', '/')).toBe('https://example.org/');
	});
	it('keeps a base path and drops the base query', () => {
		expect(networkHref('https://example.org/explorer/?x=1', '/blocks', '?p=1')).toBe(
			'https://example.org/explorer/blocks?p=1'
		);
	});
});

describe('currentNetworkIndex', () => {
	const links = parseNetworkLinks(
		'Testnet=https://testnet.example.org,Mainnet=https://example.org'
	);
	it('matches by label when one is given', () => {
		expect(currentNetworkIndex(links, 'Mainnet', 'https://testnet.example.org')).toBe(1);
	});
	it('falls back to the origin when no label is given or it matches nothing', () => {
		expect(currentNetworkIndex(links, '', 'https://testnet.example.org')).toBe(0);
		expect(currentNetworkIndex(links, 'Devnet', 'https://example.org')).toBe(1);
	});
	it('is -1 when nothing matches', () => {
		expect(currentNetworkIndex(links, '', 'http://localhost:5173')).toBe(-1);
		expect(currentNetworkIndex([], 'Testnet', 'https://example.org')).toBe(-1);
	});
});

describe('chain mismatch warning', () => {
	it('lists block time and ticker differences, and nothing for an older backend', () => {
		expect(chainMismatches({ block_seconds: 30, ticker: 'YOLO' }, 30, 'YOLO')).toEqual([]);
		expect(chainMismatches(undefined, 30, 'YOLO')).toEqual([]);
		expect(chainMismatches({ block_seconds: 30, ticker: 'YOLO' }, 120, 'ERG')).toEqual([
			'block_seconds: server 30, build 120',
			'ticker: server YOLO, build ERG'
		]);
		expect(chainMismatches({ block_seconds: 120, ticker: 'ERG' }, 120, 'YOLO')).toHaveLength(1);
	});
	it('warns on the console once, and not when the build matches', () => {
		const warn = vi.spyOn(console, 'warn').mockImplementation(() => {});
		warnOnChainMismatch({ block_seconds: BLOCK_SECONDS, ticker: TICKER });
		warnOnChainMismatch(undefined);
		expect(warn).not.toHaveBeenCalled();
		warnOnChainMismatch({ block_seconds: BLOCK_SECONDS + 1, ticker: TICKER });
		warnOnChainMismatch({ block_seconds: BLOCK_SECONDS + 1, ticker: TICKER });
		expect(warn).toHaveBeenCalledTimes(1);
		expect(String(warn.mock.calls[0][0])).toContain('block_seconds');
		warn.mockRestore();
	});
});
