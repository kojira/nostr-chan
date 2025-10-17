import { useState } from 'react';
import { Paper, Typography, Box, CircularProgress, ToggleButtonGroup, ToggleButton } from '@mui/material';
import { DatePicker } from '@mui/x-date-pickers/DatePicker';
import { LocalizationProvider } from '@mui/x-date-pickers/LocalizationProvider';
import { AdapterDayjs } from '@mui/x-date-pickers/AdapterDayjs';
import dayjs, { Dayjs } from 'dayjs';
import 'dayjs/locale/ja';
import { BarChart, Bar, XAxis, YAxis, CartesianGrid, Tooltip, Legend, ResponsiveContainer } from 'recharts';
import { useTokenUsage } from '../hooks/useTokenUsage';

dayjs.locale('ja');

const CATEGORY_COLORS: Record<string, string> = {
  'reply': '#667eea',
  'air_reply': '#9575cd',
  'summary': '#f093fb',
  'search_initial_reply': '#4facfe',
  'search_keyword_extraction': '#43e97b',
  'search_final_reply': '#fa709a',
};

const CATEGORY_LABELS: Record<string, string> = {
  'reply': 'メンション返信',
  'air_reply': 'エアリプ',
  'summary': '会話要約',
  'search_initial_reply': '検索一次回答',
  'search_keyword_extraction': 'キーワード抽出',
  'search_final_reply': '検索最終回答',
};

export const TokenUsageChart = () => {
  const [period, setPeriod] = useState<'7' | '30' | 'custom'>('7');
  const [fromDate, setFromDate] = useState<Dayjs | null>(dayjs().subtract(7, 'day'));
  const [toDate, setToDate] = useState<Dayjs | null>(dayjs());
  
  const params = period === 'custom' && fromDate && toDate
    ? { fromDate: fromDate.format('YYYY-MM-DD'), toDate: toDate.format('YYYY-MM-DD') }
    : { days: parseInt(period) };
  
  const { data, loading } = useTokenUsage(params);

  if (loading) {
    return (
      <Paper sx={{ p: 3, textAlign: 'center' }}>
        <CircularProgress />
      </Paper>
    );
  }

  // データを日付ごとに集計
  const aggregatedData: Record<string, Record<string, number>> = {};
  
  data.forEach(item => {
    if (!aggregatedData[item.date]) {
      aggregatedData[item.date] = {};
    }
    aggregatedData[item.date][item.category] = item.total_tokens;
  });

  // グラフ用のデータ形式に変換
  const chartData = Object.keys(aggregatedData)
    .sort()
    .reverse()
    .map(date => {
      const d = new Date(date);
      return {
        date: `${d.getMonth() + 1}/${d.getDate()}`,
        ...aggregatedData[date],
      };
    });

  // カテゴリの合計と回数を計算
  const categoryTotals: Record<string, number> = {};
  const categoryCounts: Record<string, number> = {};
  
  data.forEach(item => {
    if (!categoryTotals[item.category]) {
      categoryTotals[item.category] = 0;
      categoryCounts[item.category] = 0;
    }
    categoryTotals[item.category] += item.total_tokens;
    if (item.count) {
      categoryCounts[item.category] += item.count;
    }
  });

  const totalTokens = Object.values(categoryTotals).reduce((sum, val) => sum + val, 0);

  return (
    <Paper sx={{ p: 3 }}>
      <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', mb: 2, flexWrap: 'wrap', gap: 2 }}>
        <Typography variant="h6" sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
          <span>🎫</span> トークン使用量
        </Typography>
        
        <Box sx={{ display: 'flex', gap: 1, alignItems: 'center', flexWrap: 'wrap' }}>
          <ToggleButtonGroup
            value={period}
            exclusive
            onChange={(_, newPeriod) => {
              if (newPeriod !== null) {
                setPeriod(newPeriod);
              }
            }}
            size="small"
          >
            <ToggleButton value="7">7日間</ToggleButton>
            <ToggleButton value="30">30日間</ToggleButton>
            <ToggleButton value="custom">期間指定</ToggleButton>
          </ToggleButtonGroup>
          
          {period === 'custom' && (
            <LocalizationProvider dateAdapter={AdapterDayjs} adapterLocale="ja">
              <DatePicker
                label="開始日"
                value={fromDate}
                onChange={(newValue) => setFromDate(newValue)}
                slotProps={{ textField: { size: 'small', sx: { width: 140 } } }}
                maxDate={toDate || undefined}
              />
              <DatePicker
                label="終了日"
                value={toDate}
                onChange={(newValue) => setToDate(newValue)}
                slotProps={{ textField: { size: 'small', sx: { width: 140 } } }}
                minDate={fromDate || undefined}
                maxDate={dayjs()}
              />
            </LocalizationProvider>
          )}
        </Box>
      </Box>
      
      <Box sx={{ mb: 2 }}>
        <Typography variant="body2" color="text.secondary">
          合計: <strong>{totalTokens.toLocaleString()}</strong> トークン
        </Typography>
      </Box>

      {chartData.length === 0 ? (
        <Typography variant="body2" color="text.secondary" sx={{ textAlign: 'center', py: 4 }}>
          データがありません
        </Typography>
      ) : (
        <>
          <ResponsiveContainer width="100%" height={300}>
            <BarChart data={chartData}>
              <CartesianGrid strokeDasharray="3 3" />
              <XAxis dataKey="date" />
              <YAxis />
              <Tooltip 
                formatter={(value: number) => value.toLocaleString() + ' トークン'}
                labelFormatter={(label) => `日付: ${label}`}
              />
              <Legend 
                formatter={(value) => CATEGORY_LABELS[value] || value}
              />
              {Object.keys(CATEGORY_COLORS).map(category => (
                <Bar 
                  key={category}
                  dataKey={category}
                  stackId="a"
                  fill={CATEGORY_COLORS[category]}
                  name={CATEGORY_LABELS[category] || category}
                />
              ))}
            </BarChart>
          </ResponsiveContainer>

          <Box sx={{ mt: 3, display: 'flex', flexWrap: 'wrap', gap: 2 }}>
            {Object.entries(categoryTotals)
              .sort(([, a], [, b]) => b - a)
              .map(([category, total]) => {
                const count = categoryCounts[category] || 0;
                const average = count > 0 ? Math.round(total / count) : 0;
                
                return (
                  <Box key={category} sx={{ flex: '1 1 200px' }}>
                    <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, mb: 0.5 }}>
                      <Box
                        sx={{
                          width: 12,
                          height: 12,
                          borderRadius: '50%',
                          bgcolor: CATEGORY_COLORS[category] || '#999',
                        }}
                      />
                      <Typography variant="body2">
                        {CATEGORY_LABELS[category] || category}
                      </Typography>
                    </Box>
                    <Typography variant="h6" color="primary">
                      {total.toLocaleString()}
                    </Typography>
                    <Typography variant="caption" color="text.secondary" display="block">
                      割合: {((total / totalTokens) * 100).toFixed(1)}%
                    </Typography>
                    <Typography variant="caption" color="text.secondary" display="block">
                      平均: {average.toLocaleString()}トークン/回 ({count.toLocaleString()}回)
                    </Typography>
                  </Box>
                );
              })}
          </Box>
        </>
      )}
    </Paper>
  );
};

