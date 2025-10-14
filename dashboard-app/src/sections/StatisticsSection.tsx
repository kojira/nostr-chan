import { Box, Typography, Paper } from '@mui/material';
import { ShowChart } from '@mui/icons-material';
import { StatsCard } from '../components/StatsCard';
import { ReplyTrendChart } from '../components/ReplyTrendChart';
import type { Stats, BotData } from '../types';

interface StatisticsSectionProps {
  stats: Stats;
  dailyRepliesData: Record<string, Array<{ date: string; count: number }>>;
  bots: BotData[];
  dailyRepliesLoading: boolean;
}

export const StatisticsSection = ({ stats, dailyRepliesData, bots, dailyRepliesLoading }: StatisticsSectionProps) => {
  return (
    <Paper elevation={0} sx={{ p: 3, mb: 3, border: '1px solid', borderColor: 'divider', borderRadius: 2 }}>
      <Typography variant="h5" fontWeight="bold" mb={3} sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
        <ShowChart /> Bot稼働状況
        <Typography variant="caption" color="text.secondary" fontWeight="normal" ml={1}>
          リアルタイム監視
        </Typography>
      </Typography>
      
      <Box sx={{ display: 'flex', gap: 2, mb: 3 }}>
        <Box sx={{ flex: 1 }}>
          <StatsCard 
            title="稼働時間" 
            value={`${Math.floor(stats.bot_status.uptime_seconds / 60)}分`} 
            icon={ShowChart}
            color="info"
          />
        </Box>
        <Box sx={{ flex: 1 }}>
          <StatsCard 
            title="接続リレー数" 
            value={stats.bot_status.connected_relays.length} 
            icon={ShowChart}
            color="success"
          />
        </Box>
        <Box sx={{ flex: 1 }}>
          <StatsCard 
            title="ステータス" 
            value={stats.bot_status.online ? '🟢 オンライン' : '🔴 オフライン'} 
            icon={ShowChart}
            color={stats.bot_status.online ? 'success' : 'error'}
          />
        </Box>
      </Box>

      <Box sx={{ display: 'flex', gap: 2, mb: 3 }}>
        <Box sx={{ flex: 1 }}>
          <StatsCard 
            title="今日の返信" 
            value={stats.reply_stats.today} 
            icon={ShowChart}
            color="primary"
          />
        </Box>
        <Box sx={{ flex: 1 }}>
          <StatsCard 
            title="ベクトル化済" 
            value={stats.rag_stats.vectorized_events} 
            subtitle={`/ ${stats.rag_stats.total_events} イベント`}
            icon={ShowChart}
            color="secondary"
          />
        </Box>
        <Box sx={{ flex: 1 }}>
          <StatsCard 
            title="レート制限" 
            value={stats.conversation_stats.rate_limited_users} 
            subtitle="ユーザー"
            icon={ShowChart}
            color="warning"
          />
        </Box>
      </Box>

      {!dailyRepliesLoading && (
        <ReplyTrendChart data={dailyRepliesData} bots={bots} />
      )}
    </Paper>
  );
};

