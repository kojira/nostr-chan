import { useState, useEffect } from 'react';
import {
  Container, Box, Typography, IconButton, Paper, Button, TextField,
  Slider, InputAdornment
} from '@mui/material';
import { ArrowBack, Save, Chat, Timer, Numbers } from '@mui/icons-material';
import { useNavigate } from 'react-router-dom';

export const ConversationLimitSettingsPage = () => {
  const navigate = useNavigate();
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [count, setCount] = useState(5);
  const [minutes, setMinutes] = useState(3);

  useEffect(() => {
    loadSettings();
  }, []);

  const loadSettings = async () => {
    try {
      const response = await fetch('/api/settings/conversation-limit');
      if (response.ok) {
        const data = await response.json();
        setCount(data.count);
        setMinutes(data.minutes);
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
      const response = await fetch('/api/settings/conversation-limit', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          count,
          minutes,
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
          <Chat sx={{ fontSize: 32 }} />
          <Typography variant="h4" fontWeight="bold">
            会話制限設定
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

      {/* 会話制限回数 */}
      <Paper sx={{ p: 3, mb: 3 }}>
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, mb: 2 }}>
          <Numbers />
          <Typography variant="h6" fontWeight="bold">
            会話制限回数
          </Typography>
        </Box>
        <TextField
          type="number"
          value={count}
          onChange={(e) => setCount(Math.max(1, Math.min(100, parseInt(e.target.value) || 1)))}
          fullWidth
          InputProps={{
            endAdornment: <InputAdornment position="end">回</InputAdornment>,
          }}
          helperText="1〜100回の範囲で設定"
          sx={{ mb: 2 }}
        />
        <Slider
          value={count}
          onChange={(_, value) => setCount(value as number)}
          min={1}
          max={20}
          marks={[
            { value: 1, label: '1回' },
            { value: 5, label: '5回' },
            { value: 10, label: '10回' },
            { value: 15, label: '15回' },
            { value: 20, label: '20回' },
          ]}
        />
        <Paper sx={{ mt: 2, p: 2, bgcolor: 'grey.50' }}>
          <Typography variant="caption" color="text.secondary">
            💡 指定時間内に同じユーザーからの会話がこの回数を超えると、一時的に返信を停止します。
          </Typography>
        </Paper>
      </Paper>

      {/* 会話制限時間 */}
      <Paper sx={{ p: 3, mb: 3 }}>
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, mb: 2 }}>
          <Timer />
          <Typography variant="h6" fontWeight="bold">
            会話制限時間
          </Typography>
        </Box>
        <TextField
          type="number"
          value={minutes}
          onChange={(e) => setMinutes(Math.max(1, Math.min(1440, parseInt(e.target.value) || 1)))}
          fullWidth
          InputProps={{
            endAdornment: (
              <InputAdornment position="end">
                <Typography variant="caption" color="text.secondary">
                  分 {minutes >= 60 && `(≈ ${(minutes / 60).toFixed(1)}時間)`}
                </Typography>
              </InputAdornment>
            ),
          }}
          helperText="1〜1440分(24時間)の範囲で設定"
          sx={{ mb: 2 }}
        />
        <Box sx={{ display: 'flex', gap: 1, flexWrap: 'wrap' }}>
          <Button variant="outlined" onClick={() => setMinutes(1)} size="small">
            1分
          </Button>
          <Button variant="outlined" onClick={() => setMinutes(3)} size="small">
            3分
          </Button>
          <Button variant="outlined" onClick={() => setMinutes(5)} size="small">
            5分
          </Button>
          <Button variant="outlined" onClick={() => setMinutes(10)} size="small">
            10分
          </Button>
          <Button variant="outlined" onClick={() => setMinutes(30)} size="small">
            30分
          </Button>
          <Button variant="outlined" onClick={() => setMinutes(60)} size="small">
            1時間
          </Button>
        </Box>
        <Paper sx={{ mt: 2, p: 2, bgcolor: 'grey.50' }}>
          <Typography variant="caption" color="text.secondary">
            💡 会話制限の判定期間です。この時間内の会話回数をカウントします。
          </Typography>
        </Paper>
      </Paper>

      {/* 設定例 */}
      <Paper sx={{ p: 3, bgcolor: 'info.light' }}>
        <Typography variant="subtitle1" fontWeight="bold" gutterBottom>
          現在の設定
        </Typography>
        <Typography variant="body1">
          同じユーザーが <strong>{minutes}分間</strong>に<strong>{count}回以上</strong>
          会話した場合、一時的に返信を停止します。
        </Typography>
      </Paper>
    </Container>
  );
};

