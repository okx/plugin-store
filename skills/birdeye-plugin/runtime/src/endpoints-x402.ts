import type { EndpointDef } from './endpoints-apikey.js';

export const X402_ENDPOINT_KEYS = new Set<string>([
  'price',
  'history_price',
  'historical_price_unix',
  'token_trending',
  'token_overview',
  'token_security',
  'price_volume_single',
  'search_v3',
  'token_list_v3',
  'token_meme_list_v3',
  'token_meta_data_single_v3',
  'token_market_data_v3',
  'token_holder_v3',
  'token_txs_v3',
  'ohlcv_v3',
  'ohlcv_pair_v3',
  'price_stats_single_v3',
  'new_listing_v2',
  'top_traders_v2',
  'markets_v2',
  'trader_gainers_losers',
  'smart_money_list',
  'holder_distribution'
]);

export function filterX402(endpoints: EndpointDef[]): EndpointDef[] {
  return endpoints.filter((e) => X402_ENDPOINT_KEYS.has(e.key));
}
