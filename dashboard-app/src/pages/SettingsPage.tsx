import { useState, useEffect } from 'react';
import { 
  Container, Box, Typography, IconButton, Paper, TextField, Button, InputAdornment 
} from '@mui/material';
import { ArrowBack, Settings as SettingsIcon, Save, Schedule } from '@mui/icons-material';
import { useNavigate } from 'react-router-dom';

export const SettingsPage = () => {
  const navigate = useNavigate();
  const [ttlSeconds, setTtlSeconds] = useState(86400);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    loadSettings();
  }, []);

  const loadSettings = async () => {
    try {
      const response = await fetch('/api/settings/follower-cache-ttl');
      if (response.ok) {
        const data = await response.json();
        setTtlSeconds(data.ttl_seconds);
      }
    } catch (error) {
      console.error('設定読み込みエラー:', error);
    } finally {
      setLoading(false);
    }
  };

  const handleSave = async () => {
    if (ttlSeconds < 60 || ttlSeconds > 604800) {
      alert('有効時間は60秒以上604800秒(7日間)以下で設定してください');
      return;
    }

    setSaving(true);
    try {
      const response = await fetch('/api/settings/follower-cache-ttl', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ ttl_seconds: ttlSeconds }),
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

  const getHoursDisplay = () => {
    const hours = ttlSeconds / 3600;
    if (hours >= 24) {
      return `${(hours / 24).toFixed(1)}日`;
    }
    return `${hours.toFixed(1)}時間`;
  };

  if (loading) {
    return (
      <Container maxWidth="xl" sx={{ py: 4 }}>
        <Typography>読み込み中...</Typography>
      </Container>
    );
  }

  return (
    <Container maxWidth="md" sx={{ py: 4 }}>
      <Box sx={{ display: 'flex', alignItems: 'center', mb: 3 }}>
        <IconButton onClick={() => navigate('/')} sx={{ mr: 2 }}>
          <ArrowBack />
        </IconButton>
        <Typography variant="h4" fontWeight="bold">
          システム設定
        </Typography>
      </Box>

      <Paper elevation={0} sx={{ p: 3, border: '1px solid', borderColor: 'divider', borderRadius: 2 }}>
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1.5, mb: 3 }}>
          <SettingsIcon sx={{ fontSize: 32, color: 'primary.main' }} />
          <Box>
            <Typography variant="h5" fontWeight="bold">
              フォロワーキャッシュ設定
            </Typography>
            <Typography variant="caption" color="text.secondary">
              フォロワー判定結果のキャッシュ有効時間
            </Typography>
          </Box>
        </Box>

        <Box sx={{ mb: 3 }}>
          <TextField
            label="有効時間（秒）"
            type="number"
            value={ttlSeconds}
            onChange={(e) => setTtlSeconds(parseInt(e.target.value) || 0)}
            fullWidth
            InputProps={{
              startAdornment: (
                <InputAdornment position="start">
                  <Schedule />
                </InputAdornment>
              ),
              endAdornment: (
                <InputAdornment position="end">
                  <Typography variant="caption" color="text.secondary">
                    ≈ {getHoursDisplay()}
                  </Typography>
                </InputAdornment>
              ),
            }}
            helperText="最小: 60秒 / 最大: 604800秒 (7日間)"
          />
        </Box>

        <Box sx={{ display: 'flex', gap: 1 }}>
          <Button
            variant="outlined"
            onClick={() => setTtlSeconds(3600)}
            size="small"
          >
            1時間
          </Button>
          <Button
            variant="outlined"
            onClick={() => setTtlSeconds(21600)}
            size="small"
          >
            6時間
          </Button>
          <Button
            variant="outlined"
            onClick={() => setTtlSeconds(86400)}
            size="small"
          >
            24時間
          </Button>
          <Button
            variant="outlined"
            onClick={() => setTtlSeconds(604800)}
            size="small"
          >
            7日間
          </Button>
        </Box>

        <Box sx={{ mt: 3, pt: 3, borderTop: '1px solid', borderColor: 'divider' }}>
          <Button
            variant="contained"
            startIcon={<Save />}
            onClick={handleSave}
            disabled={saving || ttlSeconds < 60 || ttlSeconds > 604800}
            fullWidth
          >
            {saving ? '保存中...' : '保存'}
          </Button>
        </Box>

        <Box sx={{ mt: 2, p: 2, bgcolor: 'grey.50', borderRadius: 1 }}>
          <Typography variant="caption" color="text.secondary">
            💡 ヒント: フォロワーキャッシュはフォロワー判定の結果を一定時間保存します。
            長くすればリレーへの問い合わせが減りますが、フォロー状態の変更が反映されるまで時間がかかります。
          </Typography>
        </Box>
      </Paper>
    </Container>
  );
};

