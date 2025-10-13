import { useState, useEffect, useCallback } from 'react';
import { botApi } from '../api/botApi';
import type { BotData } from '../types';

export const useBots = () => {
  const [bots, setBots] = useState<BotData[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const loadBots = useCallback(async () => {
    try {
      setLoading(true);
      const data = await botApi.getBots();
      setBots(data);
      setError(null);
    } catch (err) {
      setError((err as Error).message);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadBots();
  }, [loadBots]);

  return { bots, loading, error, reload: loadBots };
};

