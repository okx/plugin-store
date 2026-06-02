import { ethers } from "ethers";
import { SUBRED_MANAGEMENT_ABI, ERC20_ABI } from "./abi";

/** Standard unsigned transaction payload. Can be signed and broadcast by any EVM wallet. */
export interface TxBody {
  /** Target contract address */
  to: string;
  /** Sender address */
  from: string;
  /** ABI-encoded function calldata (hex) */
  data: string;
  /** Native token value in wei (always "0" for DigiFT operations) */
  value: string;
  /** Expiry timestamp in seconds — transaction reverts if submitted after this */
  deadline: number;
  /** Human-readable summary for debugging */
  description: string;
}

/** On-chain ERC20 token balance with both raw and human-readable representations. */
export interface TokenBalance {
  /** Balance in smallest on-chain units (e.g. wei for 18-decimal tokens) */
  raw: bigint;
  /** Human-readable balance (e.g. "1.5" for 1.5 USDC) */
  formatted: string;
  /** ERC20 decimals — always read from the contract, never from API precision fields */
  decimals: number;
  /** Token symbol (e.g. "USDC") */
  symbol: string;
}

/**
 * On-chain contract client for DigiFT SubRed Management.
 *
 * Connects to an EVM-compatible RPC endpoint and provides methods to:
 * - Query ERC20 token balances and allowances.
 * - Build unsigned subscribe, redeem, and approve transactions (TxBody).
 *
 * **This client never signs or broadcasts transactions.**
 * The resulting TxBody must be signed and submitted by an EVM wallet.
 */
export class DigiFTContractClient {
  readonly provider: ethers.JsonRpcProvider;
  readonly subred: ethers.Contract;

  constructor(
    rpcUrl: string,
    subRedManagementAddress: string
  ) {
    this.provider = new ethers.JsonRpcProvider(rpcUrl);
    this.subred = new ethers.Contract(subRedManagementAddress, SUBRED_MANAGEMENT_ABI, this.provider);
  }

  private getDeadline(): number {
    return Math.floor(Date.now() / 1000) + 1800;
  }

  buildApproveTx(tokenAddress: string, spenderAddress: string, amount: bigint, from: string): TxBody {
    const token = new ethers.Contract(tokenAddress, ERC20_ABI, this.provider);
    const data = token.interface.encodeFunctionData("approve", [spenderAddress, amount]);
    return {
      to: tokenAddress,
      from,
      data,
      value: "0",
      deadline: this.getDeadline(),
      description: `ERC20 approve: ${tokenAddress} spender=${spenderAddress} amount=${amount}`,
    };
  }

  async getTokenBalance(tokenAddress: string, ownerAddress: string): Promise<TokenBalance> {
    const token = new ethers.Contract(tokenAddress, ERC20_ABI, this.provider);
    const [balance, decimals, symbol] = await Promise.all([
      token.balanceOf(ownerAddress),
      token.decimals(),
      token.symbol(),
    ]);
    return {
      raw: balance as bigint,
      formatted: ethers.formatUnits(balance, decimals),
      decimals: Number(decimals),
      symbol: symbol as string,
    };
  }

  async getAllowance(tokenAddress: string, ownerAddress: string, spenderAddress: string): Promise<bigint> {
    const token = new ethers.Contract(tokenAddress, ERC20_ABI, this.provider);
    return (await token.allowance(ownerAddress, spenderAddress)) as bigint;
  }

  /**
   * Build an unsigned subscribe transaction.
   *
   * @param stToken — ST token contract address (from API /products/{code}/chains)
   * @param currencyToken — payment token address (e.g. USDC)
   * @param amount — total raw amount including fees (use calculateTransactionFee result)
   * @param from — sender wallet address
   */
  buildSubscribeTx(stToken: string, currencyToken: string, amount: bigint, from: string): TxBody {
    const subredAddr = this.subred.target as string;
    const deadline = this.getDeadline();
    const data = this.subred.interface.encodeFunctionData("subscribe", [
      stToken,
      currencyToken,
      amount,
      deadline,
    ]);
    return {
      to: subredAddr,
      from,
      data,
      value: "0",
      deadline,
      description: `DigiFT subscribe: stToken=${stToken} currency=${currencyToken} amount=${amount}`,
    };
  }

  /**
   * Build an unsigned redeem transaction.
   *
   * @param stToken — ST token contract address
   * @param currencyToken — payout token address (e.g. USDC)
   * @param quantity — ST token quantity to redeem (raw on-chain units)
   * @param from — sender wallet address
   */
  buildRedeemTx(stToken: string, currencyToken: string, quantity: bigint, from: string): TxBody {
    const subredAddr = this.subred.target as string;
    const deadline = this.getDeadline();
    const data = this.subred.interface.encodeFunctionData("redeem", [
      stToken,
      currencyToken,
      quantity,
      deadline,
    ]);
    return {
      to: subredAddr,
      from,
      data,
      value: "0",
      deadline,
      description: `DigiFT redeem: stToken=${stToken} currency=${currencyToken} quantity=${quantity}`,
    };
  }

}
