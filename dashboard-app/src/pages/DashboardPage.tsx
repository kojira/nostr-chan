import { Container, Paper, Typography, Box, Button, Grid } from '@mui/material';
import { SmartToy, People, ChevronRight, Speed, Chat, Search, Psychology } from '@mui/icons-material';
import { useNavigate } from 'react-router-dom';
import { StatisticsSection } from '../sections/StatisticsSection';
import { useStats } from '../hooks/useStats';
import { useDailyReplies } from '../hooks/useDailyReplies';
import { useBots } from '../hooks/useBots';

export const DashboardPage = () => {
  const navigate = useNavigate();
  const { stats, loading: statsLoading } = useStats();
  const { data: dailyRepliesData, loading: dailyRepliesLoading } = useDailyReplies();
  const { bots } = useBots();

  const managementCards = [
    {
      title: 'Bot管理',
      description: `${bots.length}体のBotを管理`,
      icon: SmartToy,
      path: '/bots',
      color: '#667eea',
    },
    {
      title: 'フォロワーキャッシュ',
      description: 'フォロワー情報の管理',
      icon: People,
      path: '/follower-cache',
      color: '#764ba2',
    },
    {
      title: 'Bot動作設定',
      description: 'リアクション確率・頻度・タイムライン',
      icon: Speed,
      path: '/settings/bot-behavior',
      color: '#f093fb',
    },
    {
      title: '会話制限設定',
      description: '連続会話の制限回数・時間',
      icon: Chat,
      path: '/settings/conversation-limit',
      color: '#4facfe',
    },
    {
      title: 'RAG検索設定',
      description: '意味検索の類似度閾値',
      icon: Search,
      path: '/settings/rag',
      color: '#43e97b',
    },
    {
      title: 'GPT設定',
      description: '回答長・タイムアウト',
      icon: Psychology,
      path: '/settings/gpt',
      color: '#fa709a',
    },
  ];

  if (statsLoading) {
    return null;
  }

  return (
    <Container maxWidth="xl" sx={{ py: 4 }}>
      <StatisticsSection 
        stats={stats} 
        dailyRepliesData={dailyRepliesData}
        bots={bots}
        dailyRepliesLoading={dailyRepliesLoading}
      />

      {/* 管理機能 */}
      <Box sx={{ mb: 3 }}>
        <Typography variant="h5" fontWeight="bold" mb={2}>
          管理機能
        </Typography>
        <Grid container spacing={2}>
          {managementCards.map((card) => (
            <Grid item xs={12} sm={6} md={4} key={card.path}>
              <Paper
                elevation={0}
                sx={{
                  p: 3,
                  border: '1px solid',
                  borderColor: 'divider',
                  borderRadius: 2,
                  cursor: 'pointer',
                  transition: 'all 0.3s',
                  '&:hover': {
                    transform: 'translateY(-4px)',
                    boxShadow: '0 8px 24px rgba(0,0,0,0.12)',
                    borderColor: card.color,
                  },
                }}
                onClick={() => navigate(card.path)}
              >
                <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', mb: 2 }}>
                  <card.icon sx={{ fontSize: 40, color: card.color }} />
                  <ChevronRight sx={{ color: 'text.secondary' }} />
                </Box>
                <Typography variant="h6" fontWeight="bold" mb={0.5}>
                  {card.title}
                </Typography>
                <Typography variant="body2" color="text.secondary">
                  {card.description}
                </Typography>
              </Paper>
            </Grid>
          ))}
        </Grid>
      </Box>
    </Container>
  );
};

