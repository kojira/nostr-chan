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

  async getGlobalPause(): Promise<{ paused: boolean }> {
    const res = await fetch('/api/global-pause');
    return res.json();
  },

  async setGlobalPause(paused: boolean): Promise<{ paused: boolean }> {
    const res = await fetch('/api/global-pause', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ paused }),
    });
    if (!res.ok) throw new Error('設定に失敗しました');
    return res.json();
  },

  async getDailyReplies(): Promise<{ data: Record<string, Array<{ date: string; count: number }>> }> {
    const res = await fetch('/api/analytics/daily-replies');
    if (!res.ok) throw new Error('日別返信数の取得に失敗しました');
    return res.json();
  },
};

