import { Card, CardContent, Typography, Box } from '@mui/material';
import type { SvgIconComponent } from '@mui/icons-material';

interface StatsCardProps {
  title: string;
  value: string | number;
  subtitle?: string;
  icon?: SvgIconComponent;
  color?: 'primary' | 'success' | 'info' | 'warning' | 'error';
}

export const StatsCard = ({ title, value, subtitle, icon: Icon, color = 'primary' }: StatsCardProps) => {
  return (
    <Card 
      elevation={0}
      sx={{ 
        height: '100%', 
        transition: 'all 0.3s',
        border: '1px solid',
        borderColor: 'divider',
        background: 'linear-gradient(135deg, #ffffff 0%, #fafafa 100%)',
        '&:hover': { 
          transform: 'translateY(-4px)', 
          boxShadow: '0 8px 24px rgba(0,0,0,0.12)',
          borderColor: `${color}.main`,
        } 
      }}
    >
      <CardContent sx={{ p: 3, height: '100%', display: 'flex', flexDirection: 'column', alignItems: 'center', textAlign: 'center' }}>
        <Typography 
          variant="caption" 
          color="text.secondary" 
          fontWeight={700} 
          textTransform="uppercase" 
          letterSpacing={1.5} 
          sx={{ display: 'block', mb: 2, fontSize: '0.7rem' }}
        >
          {title}
        </Typography>
        <Typography 
          variant="h1" 
          component="div" 
          fontWeight="900" 
          sx={{ 
            lineHeight: 1.2, 
            fontSize: { xs: '2.5rem', sm: '2.75rem', md: '3rem' },
            color: `${color}.main`,
            mb: 1.5,
          }}
        >
          {value}
        </Typography>
        <Box sx={{ mt: 'auto', minHeight: '24px' }}>
          {subtitle && (
            <Typography variant="body2" color="text.secondary" sx={{ fontWeight: 500 }}>
              {subtitle}
            </Typography>
          )}
        </Box>
      </CardContent>
    </Card>
  );
};

