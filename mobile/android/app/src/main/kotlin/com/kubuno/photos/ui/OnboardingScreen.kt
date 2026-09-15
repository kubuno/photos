package com.kubuno.photos.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.systemBarsPadding
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Switch
import androidx.compose.material3.SwitchDefaults
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import com.kubuno.android.account.SharedAccount

/**
 * First-run welcome screen. Shown until the user validates it with "C'est
 * parti"; the view model persists that so it does not appear again. It presents
 * the signed-in account and offers the device auto-backup choice.
 */
@Composable
fun OnboardingScreen(account: SharedAccount, onStart: (autoBackup: Boolean) -> Unit) {
    var autoBackup by rememberSaveable { mutableStateOf(true) }
    Box(
        Modifier
            .fillMaxSize()
            .background(MaterialTheme.colorScheme.surfaceVariant)
            .systemBarsPadding()
            .padding(20.dp),
        contentAlignment = Alignment.Center,
    ) {
        Surface(
            shape = RoundedCornerShape(28.dp),
            color = MaterialTheme.colorScheme.surface,
            tonalElevation = 2.dp,
            shadowElevation = 2.dp,
            modifier = Modifier.fillMaxWidth(),
        ) {
            Column(
                Modifier.padding(horizontal = 28.dp, vertical = 32.dp),
                horizontalAlignment = Alignment.CenterHorizontally,
            ) {
                Text(
                    "Commencez à utiliser la sauvegarde Kubuno Photos",
                    style = MaterialTheme.typography.headlineSmall,
                    fontWeight = FontWeight.SemiBold,
                    textAlign = TextAlign.Center,
                    color = MaterialTheme.colorScheme.onSurface,
                )
                Spacer(Modifier.height(28.dp))
                BigAvatar(account)
                Spacer(Modifier.height(16.dp))
                Text(
                    account.displayName ?: account.label,
                    style = MaterialTheme.typography.headlineSmall,
                    fontWeight = FontWeight.SemiBold,
                )
                account.email?.let {
                    Spacer(Modifier.height(4.dp))
                    Text(it, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
                }
                Spacer(Modifier.height(20.dp))
                Text(
                    "Protégez vos souvenirs en sauvegardant vos photos et vidéos de façon sécurisée.",
                    style = MaterialTheme.typography.bodyMedium,
                    textAlign = TextAlign.Center,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                Spacer(Modifier.height(28.dp))
                Row(
                    Modifier.fillMaxWidth(),
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(16.dp),
                ) {
                    Text(
                        "Sauvegarder automatiquement les photos et vidéos de cet appareil",
                        style = MaterialTheme.typography.bodyMedium,
                        modifier = Modifier.weight(1f),
                    )
                    Switch(
                        checked = autoBackup,
                        onCheckedChange = { autoBackup = it },
                        colors = SwitchDefaults.colors(
                            checkedTrackColor = PhotosColors.Blue,
                            checkedThumbColor = Color.White,
                        ),
                    )
                }
                Spacer(Modifier.height(28.dp))
                Button(
                    onClick = { onStart(autoBackup) },
                    modifier = Modifier.fillMaxWidth(0.72f),
                    shape = PhotosShape.Pill,
                    colors = ButtonDefaults.buttonColors(containerColor = PhotosColors.Blue),
                ) {
                    Text("C'est parti", style = MaterialTheme.typography.titleMedium)
                }
                Spacer(Modifier.height(14.dp))
                Text(
                    "Vous pouvez modifier vos paramètres de sauvegarde à tout moment.",
                    style = MaterialTheme.typography.bodySmall,
                    textAlign = TextAlign.Center,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
    }
}

@Composable
private fun BigAvatar(account: SharedAccount) {
    val initial = account.label.trim().firstOrNull()?.uppercaseChar()?.toString() ?: "?"
    Surface(modifier = Modifier.size(96.dp), shape = CircleShape, color = PhotosColors.Blue) {
        Box(contentAlignment = Alignment.Center) {
            Text(
                initial,
                color = Color.White,
                style = MaterialTheme.typography.displaySmall,
                fontWeight = FontWeight.SemiBold,
            )
        }
    }
}
