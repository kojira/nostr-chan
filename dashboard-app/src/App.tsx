import { useState, useMemo, useEffect } from 'react';
import {
  Container,
  Typography,
  Box,
  Grid,
  Button,
  AppBar,
  Toolbar,
  Paper,
  CircularProgress,
  ToggleButton,
  ToggleButtonGroup,
  Switch,
  FormControlLabel,
} from '@mui/material';
import {
  SmartToy,
  Add,
  Refresh,
  CloudDone,
  CloudOff,
  ChatBubble,
  People,
  Search,
  Speed,
  CheckCircle,
  Cancel,
  ViewList,
} from '@mui/icons-material';
import { StatsCard } from './components/StatsCard';
import { BotCard } from './components/BotCard';
import { BotDialog } from './components/BotDialog';
import { ReplyTrendChart } from './components/ReplyTrendChart';
import { useBots } from './hooks/useBots';
import { useStats } from './hooks/useStats';
import { useDailyReplies } from './hooks/useDailyReplies';
import { botApi } from './api/botApi';
import type { BotData, BotRequest } from './types';

type BotFilter = 'all' | 'active' | 'inactive';

function App() {
  const { bots, loading: botsLoading, reload: reloadBots } = useBots();
  const { stats, loading: statsLoading, reload: reloadStats } = useStats();
  const { data: dailyRepliesData, loading: dailyRepliesLoading, reload: reloadDailyReplies } = useDailyReplies();
  const [dialogOpen, setDialogOpen] = useState(false);
  const [editingBot, setEditingBot] = useState<BotData | null>(null);
  const [botFilter, setBotFilter] = useState<BotFilter>('all');
  const [globalPause, setGlobalPause] = useState(false);

  useEffect(() => {
    botApi.getGlobalPause().then(({ paused }) => setGlobalPause(paused));
  }, []);

  const handleRefresh = () => {
    reloadBots();
    reloadStats();
    reloadDailyReplies();
    botApi.getGlobalPause().then(({ paused }) => setGlobalPause(paused));
  };

  const handleGlobalPauseToggle = async (event: React.ChangeEvent<HTMLInputElement>) => {
    const paused = event.target.checked;
    try {
      await botApi.setGlobalPause(paused);
      setGlobalPause(paused);
      alert(paused ? '⏸️ 全Bot一時停止を有効にしました' : '▶️ 全Bot一時停止を解除しました');
    } catch (err) {
      alert('❌ エラー: ' + (err as Error).message);
    }
  };

  const handleAddBot = () => {
    setEditingBot(null);
    setDialogOpen(true);
  };

  const handleEditBot = (bot: BotData) => {
    setEditingBot(bot);
    setDialogOpen(true);
  };

  const handleSaveBot = async (data: BotRequest, pubkey?: string) => {
    try {
      if (pubkey) {
        await botApi.updateBot(pubkey, data);
        alert('✅ Botを更新しました');
      } else {
        await botApi.createBot(data);
        alert('✅ Botを追加しました');
      }
      setDialogOpen(false);
      reloadBots();
    } catch (err) {
      alert('❌ エラー: ' + (err as Error).message);
    }
  };

  const handleDeleteBot = async (pubkey: string) => {
    if (!confirm('このBotを削除しますか？')) return;
    try {
      await botApi.deleteBot(pubkey);
      alert('✅ Botを削除しました');
      reloadBots();
    } catch (err) {
      alert('❌ エラー: ' + (err as Error).message);
    }
  };

  const handleToggleBot = async (pubkey: string) => {
    try {
      await botApi.toggleBot(pubkey);
      reloadBots();
    } catch (err) {
      alert('❌ エラー: ' + (err as Error).message);
    }
  };

  const formatUptime = (seconds: number): string => {
    const days = Math.floor(seconds / 86400);
    const hours = Math.floor((seconds % 86400) / 3600);
    const minutes = Math.floor((seconds % 3600) / 60);
    if (days > 0) return `${days}日 ${hours}時間`;
    if (hours > 0) return `${hours}時間 ${minutes}分`;
    return `${minutes}分`;
  };

  // フィルタリングされたBotリスト
  const filteredBots = useMemo(() => {
    if (botFilter === 'all') return bots;
    if (botFilter === 'active') return bots.filter(bot => bot.status === 0);
    return bots.filter(bot => bot.status === 1); // inactive
  }, [bots, botFilter]);

  return (
    <Box sx={{ flexGrow: 1, bgcolor: '#f8f9fa', minHeight: '100vh' }}>
      <AppBar 
        position="static" 
        elevation={0}
        sx={{ 
          background: 'linear-gradient(135deg, #667eea 0%, #764ba2 100%)',
          borderBottom: '1px solid rgba(255,255,255,0.1)',
        }}
      >
        <Toolbar sx={{ py: 1 }}>
          <Box sx={{ display: 'flex', alignItems: 'center', gap: 1.5 }}>
            <SmartToy sx={{ fontSize: 32 }} />
            <Box>
              <Typography variant="h6" fontWeight="bold" sx={{ lineHeight: 1.2 }}>
                Nostr Bot Dashboard
              </Typography>
              <Typography variant="caption" sx={{ opacity: 0.9 }}>
                Bot管理コンソール
              </Typography>
            </Box>
          </Box>
          <Box sx={{ flexGrow: 1 }} />
          <FormControlLabel
            control={
              <Switch
                checked={globalPause}
                onChange={handleGlobalPauseToggle}
                sx={{
                  '& .MuiSwitch-switchBase.Mui-checked': {
                    color: '#fbbf24', // 明るいアンバー（一時停止時）
                  },
                  '& .MuiSwitch-switchBase.Mui-checked + .MuiSwitch-track': {
                    backgroundColor: '#fbbf24',
                    opacity: 0.5,
                  },
                  '& .MuiSwitch-switchBase': {
                    color: '#a5f3fc', // 明るいシアン（稼働時）
                  },
                  '& .MuiSwitch-track': {
                    backgroundColor: '#a5f3fc',
                    opacity: 0.5,
                  },
                }}
              />
            }
            label={
              <Box sx={{ display: 'flex', alignItems: 'center', gap: 0.5 }}>
                {globalPause ? '⏸️' : '▶️'}
                <Typography variant="body2" fontWeight={500}>
                  {globalPause ? '一時停止中' : '稼働中'}
                </Typography>
              </Box>
            }
            sx={{ mr: 2, color: 'white' }}
          />
          <Button color="inherit" startIcon={<Refresh />} onClick={handleRefresh}>
            更新
          </Button>
        </Toolbar>
      </AppBar>

      <Container maxWidth="xl" sx={{ py: 4 }}>
        {/* Bot稼働状況 */}
        <Paper 
          elevation={0}
          sx={{ 
            p: 3, 
            mb: 4,
            border: '1px solid',
            borderColor: 'divider',
            borderRadius: 2,
          }}
        >
          <Box sx={{ display: 'flex', alignItems: 'center', gap: 2, mb: 3 }}>
            <Box 
              sx={{ 
                width: 56, 
                height: 56, 
                borderRadius: '14px',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                background: stats?.bot_status?.online 
                  ? 'linear-gradient(135deg, #10b981 0%, #059669 100%)'
                  : 'linear-gradient(135deg, #ef4444 0%, #dc2626 100%)',
                color: 'white',
              }}
            >
              {stats?.bot_status?.online ? <CloudDone sx={{ fontSize: 32 }} /> : <CloudOff sx={{ fontSize: 32 }} />}
            </Box>
            <Box>
              <Typography variant="h5" fontWeight="bold">
                Bot稼働状況
              </Typography>
              <Typography variant="body2" color="text.secondary">
                リアルタイム監視
              </Typography>
            </Box>
          </Box>
          <Grid container spacing={3}>
            <Grid item xs={12} sm={6} md={4}>
              <Typography variant="body2" color="text.secondary">稼働時間</Typography>
              <Typography variant="h6">{statsLoading ? '...' : formatUptime(stats?.bot_status?.uptime_seconds || 0)}</Typography>
            </Grid>
            <Grid item xs={12} sm={6} md={4}>
              <Typography variant="body2" color="text.secondary">接続リレー数</Typography>
              <Typography variant="h6">{statsLoading ? '...' : stats?.bot_status?.connected_relays?.length || 0}</Typography>
            </Grid>
            <Grid item xs={12} sm={6} md={4}>
              <Typography variant="body2" color="text.secondary">ステータス</Typography>
              <Typography variant="h6" color={stats?.bot_status?.online ? 'success.main' : 'error.main'}>
                {stats?.bot_status?.online ? '🟢 オンライン' : '🔴 オフライン'}
              </Typography>
            </Grid>
          </Grid>
        </Paper>

        {/* 統計カード */}
        <Box sx={{ display: 'flex', gap: 3, mb: 5 }}>
          <Box sx={{ flex: 1 }}>
            <StatsCard
              title="今日の返信"
              value={statsLoading ? '...' : stats?.reply_stats?.today || 0}
              icon={ChatBubble}
              color="primary"
            />
          </Box>
          <Box sx={{ flex: 1 }}>
            <StatsCard
              title="ベクトル化済"
              value={statsLoading ? '...' : stats?.rag_stats?.vectorized_events || 0}
              subtitle={`/ ${stats?.rag_stats?.total_events || 0} イベント`}
              icon={Search}
              color="info"
            />
          </Box>
          <Box sx={{ flex: 1 }}>
            <StatsCard
              title="レート制限"
              value={statsLoading ? '...' : stats?.conversation_stats?.rate_limited_users || 0}
              subtitle="ユーザー"
              icon={Speed}
              color="warning"
            />
          </Box>
        </Box>

        {/* グラフ */}
        {!dailyRepliesLoading && (
          <Box sx={{ mb: 5 }}>
            <ReplyTrendChart data={dailyRepliesData} bots={bots} />
          </Box>
        )}

        {/* Bot管理 */}
        <Box sx={{ mb: 4, display: 'flex', justifyContent: 'space-between', alignItems: 'center', flexWrap: 'wrap', gap: 2 }}>
          <Typography variant="h5" sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
            <SmartToy />
            Bot管理
          </Typography>
          <Box sx={{ display: 'flex', gap: 2, alignItems: 'center' }}>
            <ToggleButtonGroup
              value={botFilter}
              exclusive
              onChange={(_, newFilter) => newFilter && setBotFilter(newFilter)}
              sx={{
                '& .MuiToggleButton-root': {
                  px: 3,
                  py: 1,
                  border: '2px solid',
                  borderColor: 'divider',
                  borderRadius: '8px !important',
                  mx: 0.5,
                  fontWeight: 600,
                  '&.Mui-selected': {
                    background: 'linear-gradient(135deg, #667eea 0%, #764ba2 100%)',
                    color: 'white',
                    borderColor: '#667eea',
                    '&:hover': {
                      background: 'linear-gradient(135deg, #764ba2 0%, #667eea 100%)',
                    },
                  },
                },
              }}
            >
              <ToggleButton value="all">
                <ViewList sx={{ mr: 1, fontSize: 20 }} />
                全て ({bots.length})
              </ToggleButton>
              <ToggleButton value="active">
                <CheckCircle sx={{ mr: 1, fontSize: 20 }} />
                有効 ({bots.filter(b => b.status === 0).length})
              </ToggleButton>
              <ToggleButton value="inactive">
                <Cancel sx={{ mr: 1, fontSize: 20 }} />
                無効 ({bots.filter(b => b.status === 1).length})
              </ToggleButton>
            </ToggleButtonGroup>
            <Button variant="contained" startIcon={<Add />} onClick={handleAddBot} color="success">
              Bot追加
            </Button>
          </Box>
        </Box>

        {botsLoading ? (
          <Box sx={{ display: 'flex', justifyContent: 'center', p: 4 }}>
            <CircularProgress />
          </Box>
        ) : filteredBots.length === 0 ? (
          <Paper sx={{ p: 3, textAlign: 'center' }}>
            <Typography color="text.secondary">
              {botFilter === 'all' 
                ? '登録されているBotはありません' 
                : botFilter === 'active'
                ? '有効なBotはありません'
                : '無効なBotはありません'}
            </Typography>
          </Paper>
        ) : (
          <Grid container spacing={3}>
            {filteredBots.map((bot) => (
              <Grid key={bot.pubkey} item xs={12} md={6} lg={4}>
                <BotCard
                  bot={bot}
                  onEdit={handleEditBot}
                  onDelete={handleDeleteBot}
                  onToggle={handleToggleBot}
                />
              </Grid>
            ))}
          </Grid>
        )}
      </Container>

      <BotDialog
        open={dialogOpen}
        bot={editingBot}
        onClose={() => setDialogOpen(false)}
        onSave={handleSaveBot}
      />
    </Box>
  );
}

export default App;

