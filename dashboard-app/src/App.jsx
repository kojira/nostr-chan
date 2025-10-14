import { useState } from 'react';
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
} from '@mui/icons-material';
import { StatsCard } from './components/StatsCard';
import { BotCard } from './components/BotCard';
import { BotDialog } from './components/BotDialog';
import { useBots } from './hooks/useBots';
import { useStats } from './hooks/useStats';
import { botApi } from './api/botApi';

function App() {
  const { bots, loading: botsLoading, reload: reloadBots } = useBots();
  const { stats, loading: statsLoading, reload: reloadStats } = useStats();
  const [dialogOpen, setDialogOpen] = useState(false);
  const [editingBot, setEditingBot] = useState(null);

  const handleRefresh = () => {
    reloadBots();
    reloadStats();
  };

  const handleAddBot = () => {
    setEditingBot(null);
    setDialogOpen(true);
  };

  const handleEditBot = (bot) => {
    setEditingBot(bot);
    setDialogOpen(true);
  };

  const handleSaveBot = async (data, pubkey) => {
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
      alert('❌ エラー: ' + err.message);
    }
  };

  const handleDeleteBot = async (pubkey) => {
    if (!confirm('このBotを削除しますか？')) return;
    try {
      await botApi.deleteBot(pubkey);
      alert('✅ Botを削除しました');
      reloadBots();
    } catch (err) {
      alert('❌ エラー: ' + err.message);
    }
  };

  const handleToggleBot = async (pubkey) => {
    try {
      await botApi.toggleBot(pubkey);
      reloadBots();
    } catch (err) {
      alert('❌ エラー: ' + err.message);
    }
  };

  const formatUptime = (seconds) => {
    const days = Math.floor(seconds / 86400);
    const hours = Math.floor((seconds % 86400) / 3600);
    const minutes = Math.floor((seconds % 3600) / 60);
    if (days > 0) return `${days}日 ${hours}時間`;
    if (hours > 0) return `${hours}時間 ${minutes}分`;
    return `${minutes}分`;
  };

  return (
    <Box sx={{ flexGrow: 1, bgcolor: 'grey.50', minHeight: '100vh' }}>
      <AppBar position="static" sx={{ background: 'linear-gradient(135deg, #667eea 0%, #764ba2 100%)' }}>
        <Toolbar>
          <SmartToy sx={{ mr: 2 }} />
          <Typography variant="h6" component="div" sx={{ flexGrow: 1 }}>
            📊 Nostr Bot Dashboard
          </Typography>
          <Button color="inherit" startIcon={<Refresh />} onClick={handleRefresh}>
            更新
          </Button>
        </Toolbar>
      </AppBar>

      <Container maxWidth="xl" sx={{ py: 4 }}>
        {/* Bot稼働状況 */}
        <Paper sx={{ p: 3, mb: 4 }}>
          <Typography variant="h5" gutterBottom sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
            {stats?.bot_status?.online ? <CloudDone color="success" /> : <CloudOff color="error" />}
            Bot稼働状況
          </Typography>
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
        <Grid container spacing={3} sx={{ mb: 4 }}>
          <Grid item xs={12} sm={6} md={3}>
            <StatsCard
              title="今日の返信"
              value={statsLoading ? '...' : stats?.reply_stats?.today || 0}
              icon={ChatBubble}
              color="primary"
            />
          </Grid>
          <Grid item xs={12} sm={6} md={3}>
            <StatsCard
              title="アクティブ会話"
              value={statsLoading ? '...' : stats?.conversation_stats?.active_conversations || 0}
              icon={People}
              color="success"
            />
          </Grid>
          <Grid item xs={12} sm={6} md={3}>
            <StatsCard
              title="ベクトル化済"
              value={statsLoading ? '...' : stats?.rag_stats?.vectorized_events || 0}
              subtitle={`/ ${stats?.rag_stats?.total_events || 0} イベント`}
              icon={Search}
              color="info"
            />
          </Grid>
          <Grid item xs={12} sm={6} md={3}>
            <StatsCard
              title="レート制限"
              value={statsLoading ? '...' : stats?.conversation_stats?.rate_limited_users || 0}
              subtitle="ユーザー"
              icon={Speed}
              color="warning"
            />
          </Grid>
        </Grid>

        {/* Bot管理 */}
        <Box sx={{ mb: 3, display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
          <Typography variant="h5" sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
            <SmartToy />
            Bot管理
          </Typography>
          <Button variant="contained" startIcon={<Add />} onClick={handleAddBot} color="success">
            Bot追加
          </Button>
        </Box>

        {botsLoading ? (
          <Box sx={{ display: 'flex', justifyContent: 'center', p: 4 }}>
            <CircularProgress />
          </Box>
        ) : bots.length === 0 ? (
          <Paper sx={{ p: 3, textAlign: 'center' }}>
            <Typography color="text.secondary">登録されているBotはありません</Typography>
          </Paper>
        ) : (
          <Grid container spacing={3}>
            {bots.map((bot) => (
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

