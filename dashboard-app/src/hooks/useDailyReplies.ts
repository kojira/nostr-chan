import { useState, useEffect, useCallback } from 'react';
import { botApi } from '../api/botApi';

interface DailyReply {
  date: string;
  count: number;
}

type DailyRepliesData = Record<string, DailyReply[]>;

export const useDailyReplies = () => {
  const [data, setData] = useState<DailyRepliesData>({});
  const [loading, setLoading] = useState(true);

  const loadData = useCallback(async () => {
    try {
      setLoading(true);
      const result = await botApi.getDailyReplies();
      setData(result.data);
    } catch (err) {
      console.error('日別返信数の取得エラー:', err);
      setData({});
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadData();
  }, [loadData]);

  return { data, loading, reload: loadData };
};

