import { useMemo } from 'react';
import { 
  LineChart, 
  Line, 
  XAxis, 
  YAxis, 
  CartesianGrid, 
  Tooltip, 
  Legend, 
  ResponsiveContainer 
} from 'recharts';
import { Card, CardContent, Typography, Box } from '@mui/material';
import { TrendingUp } from '@mui/icons-material';

interface DailyReply {
  date: string;
  count: number;
}

interface BotDailyReplies {
  [botPubkey: string]: DailyReply[];
}

interface ReplyTrendChartProps {
  data: BotDailyReplies;
  bots: Array<{ pubkey: string; content: string }>;
}

// Bot毎に異なる色を割り当て
const COLORS = [
  '#667eea', // メインの紫
  '#764ba2', // 濃い紫
  '#f093fb', // ピンク
  '#4facfe', // 青
  '#43e97b', // 緑
  '#fa709a', // ローズ
  '#fee140', // 黄色
  '#30cfd0', // シアン
];

export const ReplyTrendChart = ({ data, bots }: ReplyTrendChartProps) => {
  // Bot名を取得するヘルパー
  const getBotName = (pubkey: string) => {
    const bot = bots.find(b => b.pubkey === pubkey);
    if (!bot) return pubkey.substring(0, 8) + '...';
    
    try {
      const content = JSON.parse(bot.content);
      return content.name || pubkey.substring(0, 8) + '...';
    } catch {
      return pubkey.substring(0, 8) + '...';
    }
  };

  // 全日付を取得して統合データを作成
  const chartData = useMemo(() => {
    const allDates = new Set<string>();
    
    // 全日付を収集
    Object.values(data).forEach(botData => {
      botData.forEach(({ date }) => allDates.add(date));
    });
    
    // 日付でソート
    const sortedDates = Array.from(allDates).sort();
    
    // 各日付ごとのデータを作成
    return sortedDates.map(date => {
      const dayData: any = { date };
      
      Object.keys(data).forEach(botPubkey => {
        const botDailyData = data[botPubkey].find(d => d.date === date);
        dayData[botPubkey] = botDailyData?.count || 0;
      });
      
      return dayData;
    });
  }, [data]);

  // Bot情報とカラーのマッピング
  const botKeys = Object.keys(data);

  if (botKeys.length === 0 || chartData.length === 0) {
    return (
      <Card 
        elevation={0}
        sx={{ 
          p: 3, 
          border: '1px solid',
          borderColor: 'divider',
          borderRadius: 2,
        }}
      >
        <Box sx={{ textAlign: 'center', py: 8 }}>
          <TrendingUp sx={{ fontSize: 64, color: 'text.disabled', mb: 2 }} />
          <Typography variant="h6" color="text.secondary">
            データがありません
          </Typography>
          <Typography variant="body2" color="text.disabled" sx={{ mt: 1 }}>
            Botの返信がまだ記録されていません
          </Typography>
        </Box>
      </Card>
    );
  }

  return (
    <Card 
      elevation={0}
      sx={{ 
        p: 3, 
        border: '1px solid',
        borderColor: 'divider',
        borderRadius: 2,
        background: 'linear-gradient(135deg, #ffffff 0%, #fafafa 100%)',
      }}
    >
      <CardContent>
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1.5, mb: 3 }}>
          <TrendingUp sx={{ fontSize: 32, color: 'primary.main' }} />
          <Box>
            <Typography variant="h5" fontWeight="bold">
              Bot返信推移
            </Typography>
            <Typography variant="caption" color="text.secondary">
              過去30日間のBot別返信数
            </Typography>
          </Box>
        </Box>

        <ResponsiveContainer width="100%" height={400}>
          <LineChart data={chartData}>
            <CartesianGrid strokeDasharray="3 3" stroke="#e0e0e0" />
            <XAxis 
              dataKey="date" 
              tick={{ fontSize: 12 }}
              tickFormatter={(value) => {
                // YYYY-MM-DD を MM/DD に変換
                const parts = value.split('-');
                return `${parts[1]}/${parts[2]}`;
              }}
            />
            <YAxis tick={{ fontSize: 12 }} />
            <Tooltip 
              contentStyle={{
                backgroundColor: 'rgba(255, 255, 255, 0.95)',
                border: '1px solid #e0e0e0',
                borderRadius: '8px',
                boxShadow: '0 4px 12px rgba(0,0,0,0.1)',
              }}
              labelFormatter={(value) => `日付: ${value}`}
              formatter={(value: any, name: string) => {
                return [value, getBotName(name)];
              }}
            />
            <Legend 
              formatter={(value) => getBotName(value)}
              wrapperStyle={{ paddingTop: '20px' }}
            />
            {botKeys.map((botPubkey, index) => (
              <Line
                key={botPubkey}
                type="monotone"
                dataKey={botPubkey}
                stroke={COLORS[index % COLORS.length]}
                strokeWidth={2}
                dot={{ r: 3 }}
                activeDot={{ r: 6 }}
                name={botPubkey}
              />
            ))}
          </LineChart>
        </ResponsiveContainer>
      </CardContent>
    </Card>
  );
};

