import { useState, useEffect } from 'react';
import {
  Container, Box, Typography, IconButton, Paper, Button, TextField,
  Slider, InputAdornment
} from '@mui/material';
import { ArrowBack, Save, SmartToy, Speed, Timeline, Percent } from '@mui/icons-material';
import { useNavigate } from 'react-router-dom';

export const BotBehaviorSettingsPage = () => {
  const navigate = useNavigate();
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [reactionPercent, setReactionPercent] = useState(50);
  const [reactionFreq, setReactionFreq] = useState(600);
  const [timelineSize, setTimelineSize] = useState(50);

  useEffect(() => {
    loadSettings();
  }, []);

  const loadSettings = async () => {
    try {
      const response = await fetch('/api/settings/bot-behavior');
      if (response.ok) {
        const data = await response.json();
        setReactionPercent(data.reaction_percent);
        setReactionFreq(data.reaction_freq);
        setTimelineSize(data.timeline_size);
      }
    } catch (error) {
      console.error('設定読み込みエラー:', error);
    } finally {
      setLoading(false);
    }
  };

  const handleSave = async () => {
    setSaving(true);
    try {
      const response = await fetch('/api/settings/bot-behavior', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          reaction_percent: reactionPercent,
          reaction_freq: reactionFreq,
          timeline_size: timelineSize,
        }),
      });

      if (response.ok) {
        alert('✅ 設定を保存しました');
      } else {
        alert('❌ 設定の保存に失敗しました');
      }
    } catch (error) {
      console.error('保存エラー:', error);
      alert('❌ 設定の保存に失敗しました');
    } finally {
      setSaving(false);
    }
  };

  if (loading) {
    return null;
  }

  return (
    <Container maxWidth="md" sx={{ py: 4 }}>
      {/* ヘッダー */}
      <Box sx={{ display: 'flex', alignItems: 'center', gap: 2, mb: 4 }}>
        <IconButton onClick={() => navigate('/')} size="large">
          <ArrowBack />
        </IconButton>
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, flex: 1 }}>
          <SmartToy sx={{ fontSize: 32 }} />
          <Typography variant="h4" fontWeight="bold">
            Bot動作設定
          </Typography>
        </Box>
        <Button
          variant="contained"
          startIcon={<Save />}
          onClick={handleSave}
          disabled={saving}
          size="large"
        >
          {saving ? '保存中...' : '保存'}
        </Button>
      </Box>

      {/* リアクション確率 */}
      <Paper sx={{ p: 3, mb: 3 }}>
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, mb: 2 }}>
          <Percent />
          <Typography variant="h6" fontWeight="bold">
            リアクション確率
          </Typography>
        </Box>
        <TextField
          type="number"
          value={reactionPercent}
          onChange={(e) => setReactionPercent(Math.max(0, Math.min(100, parseInt(e.target.value) || 0)))}
          fullWidth
          InputProps={{
            endAdornment: <InputAdornment position="end">%</InputAdornment>,
          }}
          helperText="0〜100%の範囲で設定"
          sx={{ mb: 2 }}
        />
        <Slider
          value={reactionPercent}
          onChange={(_, value) => setReactionPercent(value as number)}
          min={0}
          max={100}
          marks={[
            { value: 0, label: '0%' },
            { value: 25, label: '25%' },
            { value: 50, label: '50%' },
            { value: 75, label: '75%' },
            { value: 100, label: '100%' },
          ]}
        />
        <Paper sx={{ mt: 2, p: 2, bgcolor: 'grey.50' }}>
          <Typography variant="caption" color="text.secondary">
            💡 Botがエアリプ（タイムライン投稿への自発的な返信）を行う確率です。メンションへは常に100%反応します。
          </Typography>
        </Paper>
      </Paper>

      {/* リアクション頻度 */}
      <Paper sx={{ p: 3, mb: 3 }}>
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, mb: 2 }}>
          <Speed />
          <Typography variant="h6" fontWeight="bold">
            リアクション頻度
          </Typography>
        </Box>
        <TextField
          type="number"
          value={reactionFreq}
          onChange={(e) => setReactionFreq(Math.max(1, parseInt(e.target.value) || 1))}
          fullWidth
          InputProps={{
            endAdornment: (
              <InputAdornment position="end">
                <Typography variant="caption" color="text.secondary">
                  秒 (≈ {(reactionFreq / 60).toFixed(1)}分)
                </Typography>
              </InputAdornment>
            ),
          }}
          helperText="最小: 1秒"
          sx={{ mb: 2 }}
        />
        <Box sx={{ display: 'flex', gap: 1, flexWrap: 'wrap' }}>
          <Button variant="outlined" onClick={() => setReactionFreq(60)} size="small">
            1分
          </Button>
          <Button variant="outlined" onClick={() => setReactionFreq(300)} size="small">
            5分
          </Button>
          <Button variant="outlined" onClick={() => setReactionFreq(600)} size="small">
            10分
          </Button>
          <Button variant="outlined" onClick={() => setReactionFreq(1800)} size="small">
            30分
          </Button>
        </Box>
        <Paper sx={{ mt: 2, p: 2, bgcolor: 'grey.50' }}>
          <Typography variant="caption" color="text.secondary">
            💡 タイムラインをチェックする間隔です。短いほど頻繁にチェックします。
          </Typography>
        </Paper>
      </Paper>

      {/* タイムラインサイズ */}
      <Paper sx={{ p: 3, mb: 3 }}>
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, mb: 2 }}>
          <Timeline />
          <Typography variant="h6" fontWeight="bold">
            タイムラインサイズ
          </Typography>
        </Box>
        <TextField
          type="number"
          value={timelineSize}
          onChange={(e) => setTimelineSize(Math.max(1, Math.min(1000, parseInt(e.target.value) || 1)))}
          fullWidth
          InputProps={{
            endAdornment: <InputAdornment position="end">件</InputAdornment>,
          }}
          helperText="1〜1000件の範囲で設定"
          sx={{ mb: 2 }}
        />
        <Slider
          value={timelineSize}
          onChange={(_, value) => setTimelineSize(value as number)}
          min={1}
          max={200}
          marks={[
            { value: 1, label: '1' },
            { value: 50, label: '50' },
            { value: 100, label: '100' },
            { value: 150, label: '150' },
            { value: 200, label: '200' },
          ]}
        />
        <Paper sx={{ mt: 2, p: 2, bgcolor: 'grey.50' }}>
          <Typography variant="caption" color="text.secondary">
            💡 一度に取得するタイムラインの投稿数です。多いほど広範囲を監視できますが、負荷が増えます。
          </Typography>
        </Paper>
      </Paper>
    </Container>
  );
};

