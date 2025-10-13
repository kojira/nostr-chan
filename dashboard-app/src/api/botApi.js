// Bot API
export const botApi = {
  async getStats() {
    const res = await fetch('/api/stats');
    return res.json();
  },

  async getBots() {
    const res = await fetch('/api/bots');
    return res.json();
  },

  async createBot(data) {
    const res = await fetch('/api/bots', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(data),
    });
    if (!res.ok) throw new Error(await res.text());
    return res.json();
  },

  async updateBot(pubkey, data) {
    const res = await fetch(`/api/bots/${pubkey}`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(data),
    });
    if (!res.ok) throw new Error(await res.text());
    return res.json();
  },

  async deleteBot(pubkey) {
    const res = await fetch(`/api/bots/${pubkey}`, {
      method: 'DELETE',
    });
    if (!res.ok) throw new Error('削除に失敗しました');
  },

  async toggleBot(pubkey) {
    const res = await fetch(`/api/bots/${pubkey}/toggle`, {
      method: 'POST',
    });
    if (!res.ok) throw new Error('切り替えに失敗しました');
    return res.json();
  },
};

