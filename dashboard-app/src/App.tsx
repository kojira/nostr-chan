import { useState, useEffect } from 'react';
import { BrowserRouter, Routes, Route, useLocation, useNavigate } from 'react-router-dom';
import {
  Box,
  Button,
  AppBar,
  Toolbar,
  Typography,
  Switch,
  CircularProgress,
} from '@mui/material';
import { SmartToy, Refresh } from '@mui/icons-material';
import { DashboardPage } from './pages/DashboardPage';
import { BotsPage } from './pages/BotsPage';
import { BotDetailPage } from './pages/BotDetailPage';
import { BotSummariesPage } from './pages/BotSummariesPage';
import { EventsPage } from './pages/EventsPage';
import { FollowerCachePage } from './pages/FollowerCachePage';
import { BotBehaviorSettingsPage } from './pages/BotBehaviorSettingsPage';
import { ConversationLimitSettingsPage } from './pages/ConversationLimitSettingsPage';
import { RagSettingsPage } from './pages/RagSettingsPage';
import { GptSettingsPage } from './pages/GptSettingsPage';
import { RelaySettingsPage } from './pages/RelaySettingsPage';
import { BlacklistSettingsPage } from './pages/BlacklistSettingsPage';
import TokenDetailsPage from './pages/TokenDetailsPage';
import { useBots } from './hooks/useBots';
import { useStats } from './hooks/useStats';
import { useDailyReplies } from './hooks/useDailyReplies';
import { botApi } from './api/botApi';

function AppContent() {
  const location = useLocation();
  const navigate = useNavigate();
  const { loading: botsLoading, reload: reloadBots } = useBots();
  const { loading: statsLoading, reload: reloadStats } = useStats();
  const { reload: reloadDailyReplies } = useDailyReplies();
  const [globalPause, setGlobalPause] = useState(false);
  const [pauseLoading, setPauseLoading] = useState(false);

  useEffect(() => {
    const loadGlobalPause = async () => {
      try {
        const paused = await botApi.getGlobalPause();
        setGlobalPause(paused);
      } catch (error) {
        console.error('グローバル一時停止状態の取得エラー:', error);
      }
    };
    loadGlobalPause();
  }, []);

  const handleRefresh = () => {
    reloadBots();
    reloadStats();
    reloadDailyReplies();
  };

  const handleGlobalPauseToggle = async () => {
    setPauseLoading(true);
    try {
      await botApi.setGlobalPause(!globalPause);
      setGlobalPause(!globalPause);
      alert(!globalPause ? '⏸️ 全Bot一時停止しました' : '▶️ 全Bot一時停止を解除しました');
    } catch (error) {
      console.error('グローバル一時停止の切替エラー:', error);
      alert('❌ 切替に失敗しました');
    } finally {
      setPauseLoading(false);
    }
  };

  // トップページ以外では更新ボタンを非表示
  const showRefreshButton = location.pathname === '/';

  if (botsLoading || statsLoading) {
    return (
      <Box sx={{ display: 'flex', justifyContent: 'center', alignItems: 'center', height: '100vh' }}>
        <CircularProgress />
      </Box>
    );
  }

  return (
    <Box sx={{ minHeight: '100vh', bgcolor: '#f5f5f7' }}>
      <AppBar 
        position="static" 
        elevation={0}
        sx={{ 
          background: 'linear-gradient(135deg, #667eea 0%, #764ba2 100%)',
          borderBottom: '1px solid rgba(255,255,255,0.1)',
        }}
      >
        <Toolbar>
          <SmartToy sx={{ mr: 2, fontSize: 32 }} />
          <Box 
            onClick={() => navigate('/')}
            sx={{ 
              cursor: 'pointer',
              '&:hover': { opacity: 0.8 },
              transition: 'opacity 0.2s'
            }}
          >
            <Typography variant="h6" fontWeight="bold">
              📊 Nostr Bot Dashboard
            </Typography>
            <Typography variant="caption" sx={{ opacity: 0.9 }}>
              Bot管理コンソール
            </Typography>
          </Box>
          <Box sx={{ flexGrow: 1 }} />
          <Box sx={{ display: 'flex', gap: 2, alignItems: 'center' }}>
            <Switch
              checked={!globalPause}
              onChange={handleGlobalPauseToggle}
              disabled={pauseLoading}
              sx={{
                '& .MuiSwitch-switchBase.Mui-checked': {
                  color: '#a5f3fc',
                },
                '& .MuiSwitch-switchBase.Mui-checked + .MuiSwitch-track': {
                  backgroundColor: '#a5f3fc',
                  opacity: 0.5,
                },
                '& .MuiSwitch-switchBase': {
                  color: '#fbbf24',
                },
                '& .MuiSwitch-track': {
                  backgroundColor: '#fbbf24',
                  opacity: 0.5,
                },
              }}
            />
            <Typography variant="body2" fontWeight="medium">
              {globalPause ? '⏸️ 一時停止中' : '▶️ 稼働中'}
            </Typography>
            {showRefreshButton && (
              <Button
                variant="outlined"
                startIcon={<Refresh />}
                onClick={handleRefresh}
                sx={{
                  color: 'white',
                  borderColor: 'rgba(255,255,255,0.3)',
                  '&:hover': {
                    borderColor: 'white',
                    bgcolor: 'rgba(255,255,255,0.1)',
                  },
                }}
              >
                更新
              </Button>
            )}
          </Box>
        </Toolbar>
      </AppBar>

      <Routes>
        <Route path="/" element={<DashboardPage />} />
        <Route path="/bots" element={<BotsPage />} />
        <Route path="/bots/:pubkey" element={<BotDetailPage />} />
        <Route path="/bots/:pubkey/summaries" element={<BotSummariesPage />} />
        <Route path="/events" element={<EventsPage />} />
        <Route path="/follower-cache" element={<FollowerCachePage />} />
        <Route path="/analytics/token-details" element={<TokenDetailsPage />} />
        <Route path="/settings/bot-behavior" element={<BotBehaviorSettingsPage />} />
        <Route path="/settings/conversation-limit" element={<ConversationLimitSettingsPage />} />
        <Route path="/settings/rag" element={<RagSettingsPage />} />
        <Route path="/settings/gpt" element={<GptSettingsPage />} />
        <Route path="/settings/relay" element={<RelaySettingsPage />} />
        <Route path="/settings/blacklist" element={<BlacklistSettingsPage />} />
      </Routes>
    </Box>
  );
}

function App() {
  return (
    <BrowserRouter>
      <AppContent />
    </BrowserRouter>
  );
}

export default App;
