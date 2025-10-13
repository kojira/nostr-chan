import { useState, useEffect, useCallback } from 'react';
import { botApi } from '../api/botApi';

export const useBots = () => {
  const [bots, setBots] = useState([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(null);

  const loadBots = useCallback(async () => {
    try {
      setLoading(true);
      const data = await botApi.getBots();
      setBots(data);
      setError(null);
    } catch (err) {
      setError(err.message);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadBots();
  }, [loadBots]);

  return { bots, loading, error, reload: loadBots };
};

