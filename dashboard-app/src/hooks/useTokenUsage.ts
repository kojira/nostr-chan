import { useState, useEffect } from 'react';

export interface TokenUsageData {
  date: string;
  category: string;
  total_tokens: number;
  count?: number;
}

interface UseTokenUsageParams {
  days?: number;
  fromDate?: string;
  toDate?: string;
}

export const useTokenUsage = (params: UseTokenUsageParams) => {
  const [data, setData] = useState<TokenUsageData[]>([]);
  const [loading, setLoading] = useState(true);

  const fetchData = async () => {
    try {
      let url = '/api/analytics/token-usage';
      const queryParams = new URLSearchParams();
      
      if (params.fromDate && params.toDate) {
        queryParams.append('from', params.fromDate);
        queryParams.append('to', params.toDate);
      } else if (params.days) {
        queryParams.append('days', params.days.toString());
      }
      
      if (queryParams.toString()) {
        url += `?${queryParams.toString()}`;
      }
      
      const response = await fetch(url);
      if (response.ok) {
        const result = await response.json();
        setData(result.data || []);
      }
    } catch (error) {
      console.error('トークン使用量取得エラー:', error);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    setLoading(true);
    fetchData();
    const interval = setInterval(fetchData, 60000); // 1分ごとに更新
    return () => clearInterval(interval);
  }, [params.days, params.fromDate, params.toDate]);

  return { data, loading };
};

