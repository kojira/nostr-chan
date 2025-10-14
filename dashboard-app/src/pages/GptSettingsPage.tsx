import { useState, useEffect } from 'react';
import {
  Container, Box, Typography, IconButton, Paper, Button, TextField,
  Slider, InputAdornment
} from '@mui/material';
import { ArrowBack, Save, Psychology, Timer, TextFields } from '@mui/icons-material';
import { useNavigate } from 'react-router-dom';

export const GptSettingsPage = () => {
  const navigate = useNavigate();
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [answerLength, setAnswerLength] = useState(100);
  const [timeout, setTimeout] = useState(60);

  useEffect(() => {
    loadSettings();
  }, []);

  const loadSettings = async () => {
    try {
      const response = await fetch('/api/settings/gpt');
      if (response.ok) {
        const data = await response.json();
        setAnswerLength(data.answer_length);
        setTimeout(data.timeout);
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
      const response = await fetch('/api/settings/gpt', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          answer_length: answerLength,
          timeout: timeout,
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
          <Psychology sx={{ fontSize: 32 }} />
          <Typography variant="h4" fontWeight="bold">
            GPT設定
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

      {/* 回答長 */}
      <Paper sx={{ p: 3, mb: 3 }}>
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, mb: 2 }}>
          <TextFields />
          <Typography variant="h6" fontWeight="bold">
            回答長
          </Typography>
        </Box>
        <TextField
          type="number"
          value={answerLength}
          onChange={(e) => setAnswerLength(Math.max(10, Math.min(1000, parseInt(e.target.value) || 10)))}
          fullWidth
          InputProps={{
            endAdornment: <InputAdornment position="end">文字</InputAdornment>,
          }}
          helperText="10〜1000文字の範囲で設定"
          sx={{ mb: 2 }}
        />
        <Slider
          value={answerLength}
          onChange={(_, value) => setAnswerLength(value as number)}
          min={10}
          max={500}
          marks={[
            { value: 10, label: '10' },
            { value: 100, label: '100' },
            { value: 200, label: '200' },
            { value: 300, label: '300' },
            { value: 500, label: '500' },
          ]}
        />
        <Box sx={{ display: 'flex', gap: 1, mt: 2, flexWrap: 'wrap' }}>
          <Button variant="outlined" onClick={() => setAnswerLength(50)} size="small">
            短め (50文字)
          </Button>
          <Button variant="outlined" onClick={() => setAnswerLength(100)} size="small">
            普通 (100文字)
          </Button>
          <Button variant="outlined" onClick={() => setAnswerLength(200)} size="small">
            長め (200文字)
          </Button>
          <Button variant="outlined" onClick={() => setAnswerLength(300)} size="small">
            詳細 (300文字)
          </Button>
        </Box>
        <Paper sx={{ mt: 2, p: 2, bgcolor: 'grey.50' }}>
          <Typography variant="caption" color="text.secondary">
            💡 Botが生成する返信の目安となる文字数です。実際の返信はこれより多少前後します。
          </Typography>
        </Paper>
      </Paper>

      {/* タイムアウト */}
      <Paper sx={{ p: 3, mb: 3 }}>
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, mb: 2 }}>
          <Timer />
          <Typography variant="h6" fontWeight="bold">
            タイムアウト
          </Typography>
        </Box>
        <TextField
          type="number"
          value={timeout}
          onChange={(e) => setTimeout(Math.max(10, Math.min(300, parseInt(e.target.value) || 10)))}
          fullWidth
          InputProps={{
            endAdornment: <InputAdornment position="end">秒</InputAdornment>,
          }}
          helperText="10〜300秒の範囲で設定"
          sx={{ mb: 2 }}
        />
        <Slider
          value={timeout}
          onChange={(_, value) => setTimeout(value as number)}
          min={10}
          max={180}
          marks={[
            { value: 10, label: '10秒' },
            { value: 30, label: '30秒' },
            { value: 60, label: '60秒' },
            { value: 120, label: '120秒' },
            { value: 180, label: '180秒' },
          ]}
        />
        <Box sx={{ display: 'flex', gap: 1, mt: 2, flexWrap: 'wrap' }}>
          <Button variant="outlined" onClick={() => setTimeout(30)} size="small">
            30秒
          </Button>
          <Button variant="outlined" onClick={() => setTimeout(60)} size="small">
            1分
          </Button>
          <Button variant="outlined" onClick={() => setTimeout(90)} size="small">
            1.5分
          </Button>
          <Button variant="outlined" onClick={() => setTimeout(120)} size="small">
            2分
          </Button>
        </Box>
        <Paper sx={{ mt: 2, p: 2, bgcolor: 'grey.50' }}>
          <Typography variant="caption" color="text.secondary">
            💡 GPT APIからの応答を待つ最大時間です。長すぎるとBotの応答が遅くなります。
          </Typography>
        </Paper>
      </Paper>

      {/* 設定例 */}
      <Paper sx={{ p: 3, bgcolor: 'info.light' }}>
        <Typography variant="subtitle1" fontWeight="bold" gutterBottom>
          現在の設定
        </Typography>
        <Typography variant="body1">
          Botは<strong>約{answerLength}文字</strong>の返信を生成し、
          <strong>{timeout}秒</strong>以内に応答がない場合はタイムアウトします。
        </Typography>
      </Paper>
    </Container>
  );
};

