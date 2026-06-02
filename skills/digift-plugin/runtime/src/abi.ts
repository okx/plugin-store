export const SUBRED_MANAGEMENT_ABI = [
  "function subscribe(address stToken, address currencyToken, uint amount, uint _deadline) external",
  "function redeem(address stToken, address currencyToken, uint quantity, uint deadline) external",
] as const;

export const ERC20_ABI = [
  "function balanceOf(address owner) external view returns (uint256)",
  "function allowance(address owner, address spender) external view returns (uint256)",
  "function approve(address spender, uint256 amount) external returns (bool)",
  "function symbol() external view returns (string)",
  "function decimals() external view returns (uint8)",
] as const;
