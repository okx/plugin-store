export type EndpointDef = {
  key: string;
  path: string;
  required?: string[];
};

export const APIKEY_ENDPOINTS: EndpointDef[] = [
  { key: 'price', path: '/defi/price', required: ['address'] },
  { key: 'multi_price', path: '/defi/multi_price', required: ['list_address'] },
  { key: 'history_price', path: '/defi/history_price', required: ['address', 'address_type', 'type', 'time_from', 'time_to'] },
  { key: 'historical_price_unix', path: '/defi/historical_price_unix', required: ['address', 'unixtime'] },
  { key: 'token_trending', path: '/defi/token_trending' },
  { key: 'token_overview', path: '/defi/token_overview', required: ['address'] },
  { key: 'token_security', path: '/defi/token_security', required: ['address'] },
  { key: 'price_volume_single', path: '/defi/price_volume/single', required: ['address', 'type'] },
  { key: 'search_v3', path: '/defi/v3/search' },
  { key: 'token_list_v3', path: '/defi/v3/token/list' },
  { key: 'token_meme_list_v3', path: '/defi/v3/token/meme/list' },
  { key: 'token_meta_data_single_v3', path: '/defi/v3/token/meta-data/single', required: ['address'] },
  { key: 'token_market_data_v3', path: '/defi/v3/token/market-data', required: ['address'] },
  { key: 'token_trade_data_single_v3', path: '/defi/v3/token/trade-data/single', required: ['address'] },
  { key: 'token_holder_v3', path: '/defi/v3/token/holder', required: ['address'] },
  { key: 'token_txs_v3', path: '/defi/v3/token/txs', required: ['address'] },
  { key: 'ohlcv_v3', path: '/defi/v3/ohlcv', required: ['address', 'type', 'time_from', 'time_to'] },
  { key: 'ohlcv_pair_v3', path: '/defi/v3/ohlcv/pair', required: ['address', 'type', 'time_from', 'time_to'] },
  { key: 'price_stats_single_v3', path: '/defi/v3/price/stats/single', required: ['address'] },
  { key: 'new_listing_v2', path: '/defi/v2/tokens/new_listing' },
  { key: 'top_traders_v2', path: '/defi/v2/tokens/top_traders', required: ['address', 'time_frame'] },
  { key: 'markets_v2', path: '/defi/v2/markets', required: ['address', 'time_frame'] },
  { key: 'trader_gainers_losers', path: '/trader/gainers-losers', required: ['type'] },
  { key: 'smart_money_list', path: '/smart-money/v1/token/list' },
  { key: 'holder_distribution', path: '/holder/v1/distribution', required: ['token_address'] }
];
