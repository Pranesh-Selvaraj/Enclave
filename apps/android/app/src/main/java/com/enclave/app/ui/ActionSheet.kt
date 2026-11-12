package com.enclave.app.ui

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp

/**
 * One row of an [ActionSheet].
 *
 * [enabled] false renders the row dimmed and inert, so an action that does not
 * apply (sharing a note to widgets when the vault is locked, say) is visible
 * but cannot be tapped. [destructive] tints the row with the error colour for
 * delete-style actions.
 */
data class ActionItem(
    val icon: ImageVector,
    val label: String,
    val enabled: Boolean = true,
    val destructive: Boolean = false,
    val onClick: () -> Unit,
)

/**
 * A bottom sheet listing actions for one item — a note or a folder — opened by
 * a long press. It is the phone's equivalent of a context menu: the actions are
 * fixed per item type, so they are discoverable instead of hidden behind a
 * gesture nobody tries.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ActionSheet(
    title: String,
    subtitle: String? = null,
    actions: List<ActionItem>,
    onDismiss: () -> Unit,
) {
    ModalBottomSheet(
        onDismissRequest = onDismiss,
        containerColor = MaterialTheme.colorScheme.surface,
    ) {
        Column(Modifier.fillMaxWidth().padding(bottom = 12.dp)) {
            Text(
                title,
                style = MaterialTheme.typography.titleMedium,
                fontWeight = FontWeight.SemiBold,
                maxLines = 2,
                overflow = TextOverflow.Ellipsis,
                modifier = Modifier.padding(horizontal = 24.dp, vertical = 4.dp),
            )
            subtitle?.let {
                Spacer(Modifier.height(2.dp))
                Text(
                    it,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.padding(horizontal = 24.dp).padding(bottom = 6.dp),
                )
            }
            Spacer(Modifier.height(8.dp))
            actions.forEach { action ->
                val tint = when {
                    !action.enabled -> MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.4f)
                    action.destructive -> MaterialTheme.colorScheme.error
                    else -> MaterialTheme.colorScheme.onSurface
                }
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .clickable(enabled = action.enabled) {
                            onDismiss()
                            action.onClick()
                        }
                        .padding(horizontal = 24.dp, vertical = 14.dp),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Icon(action.icon, contentDescription = null, tint = tint)
                    Spacer(Modifier.width(16.dp))
                    Text(
                        action.label,
                        style = MaterialTheme.typography.bodyLarge,
                        color = tint,
                    )
                }
            }
        }
    }
}
