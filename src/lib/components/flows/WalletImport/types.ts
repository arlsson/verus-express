export type ImportMethod = 'seed24' | 'text';

export type WalletData = {
  name: string;
  emoji: string;
  color: string;
  password: string;
  network: 'mainnet' | 'testnet';
};

export type WalletUpdate = Partial<WalletData>;
