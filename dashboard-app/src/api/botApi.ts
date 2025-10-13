import type { Stats, BotData, BotRequest } from '../types';

// Bot API
export const botApi = {
  async getStats(): Promise<Stats> {
    const res = await fetch('/api/stats');
    return res.json();
  },

  async getBots(): Promise<BotData[]> {
    const res = await fetch('/api/bots');
    return res.json();
  },

  async createBot(data: BotRequest): Promise<BotData> {
    const res = await fetch('/api/bots', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(data),
    });
    if (!res.ok) throw new Error(await res.text());
    return res.json();
  },

  async updateBot(pubkey: string, data: BotRequest): Promise<BotData> {
    const res = await fetch(`/api/bots/${pubkey}`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(data),
    });
    if (!res.ok) throw new Error(await res.text());
    return res.json();
  },

  async deleteBot(pubkey: string): Promise<void> {
    const res = await fetch(`/api/bots/${pubkey}`, {
      method: 'DELETE',
    });
    if (!res.ok) throw new Error('削除に失敗しました');
  },

  async toggleBot(pubkey: string): Promise<BotData> {
    const res = await fetch(`/api/bots/${pubkey}/toggle`, {
      method: 'POST',
    });
    if (!res.ok) throw new Error('切り替えに失敗しました');
    return res.json();
  },
};

