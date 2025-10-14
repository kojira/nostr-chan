import { Box, Typography, Paper } from '@mui/material';
import { AccessTime, Wifi, Circle, ChatBubble, Storage, Block } from '@mui/icons-material';
import { StatsCard } from '../components/StatsCard';
import { ReplyTrendChart } from '../components/ReplyTrendChart';
import type { Stats, BotData } from '../types';

interface StatisticsSectionProps {
  stats: Stats;
  dailyRepliesData: Record<string, Array<{ date: string; count: number }>>;
  bots: BotData[];
  dailyRepliesLoading: boolean;
}

const formatUptime = (seconds: number): string => {
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  if (hours > 0) {
    return `${hours}時間${minutes}分`;
  }
  return `${minutes}分`;
};

export const StatisticsSection = ({ stats, dailyRepliesData, bots, dailyRepliesLoading }: StatisticsSectionProps) => {
  return (
    <Box sx={{ mb: 3 }}>
      <Typography variant="h5" fontWeight="bold" mb={2}>
        Bot稼働状況
      </Typography>
      
      <Paper elevation={0} sx={{ p: 2, border: '1px solid', borderColor: 'divider', borderRadius: 2 }}>
        <Box sx={{ display: 'grid', gridTemplateColumns: { xs: 'repeat(2, 1fr)', sm: 'repeat(3, 1fr)', md: 'repeat(6, 1fr)' }, gap: 2 }}>
          <Box sx={{ textAlign: 'center' }}>
            <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'center', gap: 0.5, mb: 0.5 }}>
              <AccessTime sx={{ fontSize: 16, color: 'info.main' }} />
              <Typography variant="caption" color="text.secondary">稼働時間</Typography>
            </Box>
            <Typography variant="h6" fontWeight="bold" color="info.main">
              {formatUptime(stats.bot_status.uptime_seconds)}
            </Typography>
          </Box>

          <Box sx={{ textAlign: 'center' }}>
            <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'center', gap: 0.5, mb: 0.5 }}>
              <Wifi sx={{ fontSize: 16, color: 'success.main' }} />
              <Typography variant="caption" color="text.secondary">接続リレー</Typography>
            </Box>
            <Typography variant="h6" fontWeight="bold" color="success.main">
              {stats.bot_status.connected_relays.length}
            </Typography>
          </Box>

          <Box sx={{ textAlign: 'center' }}>
            <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'center', gap: 0.5, mb: 0.5 }}>
              <Circle sx={{ fontSize: 16, color: stats.bot_status.online ? 'success.main' : 'error.main' }} />
              <Typography variant="caption" color="text.secondary">ステータス</Typography>
            </Box>
            <Typography variant="h6" fontWeight="bold" color={stats.bot_status.online ? 'success.main' : 'error.main'}>
              {stats.bot_status.online ? 'オンライン' : 'オフライン'}
            </Typography>
          </Box>

          <Box sx={{ textAlign: 'center' }}>
            <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'center', gap: 0.5, mb: 0.5 }}>
              <ChatBubble sx={{ fontSize: 16, color: 'primary.main' }} />
              <Typography variant="caption" color="text.secondary">今日の返信</Typography>
            </Box>
            <Typography variant="h6" fontWeight="bold" color="primary.main">
              {stats.reply_stats.today}
            </Typography>
          </Box>

          <Box sx={{ textAlign: 'center' }}>
            <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'center', gap: 0.5, mb: 0.5 }}>
              <Storage sx={{ fontSize: 16, color: 'secondary.main' }} />
              <Typography variant="caption" color="text.secondary">ベクトル化</Typography>
            </Box>
            <Typography variant="h6" fontWeight="bold" color="secondary.main">
              {stats.rag_stats.vectorized_events}
            </Typography>
            <Typography variant="caption" color="text.secondary">
              / {stats.rag_stats.total_events}
            </Typography>
          </Box>

          <Box sx={{ textAlign: 'center' }}>
            <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'center', gap: 0.5, mb: 0.5 }}>
              <Block sx={{ fontSize: 16, color: 'warning.main' }} />
              <Typography variant="caption" color="text.secondary">レート制限</Typography>
            </Box>
            <Typography variant="h6" fontWeight="bold" color="warning.main">
              {stats.conversation_stats.rate_limited_users}
            </Typography>
          </Box>
        </Box>
      </Paper>
    </Box>
  );
};

// グラフコンポーネントを分離
export const ReplyTrendSection = ({ dailyRepliesData, bots, dailyRepliesLoading }: Omit<StatisticsSectionProps, 'stats'>) => {
  if (dailyRepliesLoading) {
    return null;
  }

  return (
    <Paper elevation={0} sx={{ p: 3, mb: 3, border: '1px solid', borderColor: 'divider', borderRadius: 2 }}>
      <ReplyTrendChart data={dailyRepliesData} bots={bots} />
    </Paper>
  );
};

