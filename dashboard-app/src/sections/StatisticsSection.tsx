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
    <>
      {/* Bot稼働状況 */}
      <Box sx={{ mb: 3 }}>
        <Typography variant="h5" fontWeight="bold" mb={2}>
          Bot稼働状況
        </Typography>
        
        <Box sx={{ display: 'grid', gridTemplateColumns: { xs: '1fr', sm: 'repeat(2, 1fr)', md: 'repeat(3, 1fr)' }, gap: 2 }}>
          <StatsCard 
            title="稼働時間" 
            value={formatUptime(stats.bot_status.uptime_seconds)} 
            icon={AccessTime}
            color="info"
          />
          <StatsCard 
            title="接続リレー数" 
            value={stats.bot_status.connected_relays.length} 
            icon={Wifi}
            color="success"
          />
          <StatsCard 
            title="ステータス" 
            value={stats.bot_status.online ? 'オンライン' : 'オフライン'} 
            icon={Circle}
            color={stats.bot_status.online ? 'success' : 'error'}
          />
          <StatsCard 
            title="今日の返信" 
            value={stats.reply_stats.today} 
            icon={ChatBubble}
            color="primary"
          />
          <StatsCard 
            title="ベクトル化済" 
            value={stats.rag_stats.vectorized_events} 
            subtitle={`/ ${stats.rag_stats.total_events} イベント`}
            icon={Storage}
            color="secondary"
          />
          <StatsCard 
            title="レート制限" 
            value={stats.conversation_stats.rate_limited_users} 
            subtitle="ユーザー"
            icon={Block}
            color="warning"
          />
        </Box>
      </Box>

      {/* Bot返信推移 */}
      {!dailyRepliesLoading && (
        <Paper elevation={0} sx={{ p: 3, mb: 3, border: '1px solid', borderColor: 'divider', borderRadius: 2 }}>
          <ReplyTrendChart data={dailyRepliesData} bots={bots} />
        </Paper>
      )}
    </>
  );
};

