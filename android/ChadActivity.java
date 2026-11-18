package org.chad.runtime;

import android.app.Activity;
import android.app.AlertDialog;
import android.content.ActivityNotFoundException;
import android.content.ComponentName;
import android.net.Uri;
import android.os.Bundle;
import android.os.Handler;
import android.os.Looper;
import android.view.Gravity;
import android.widget.TextView;

import androidx.browser.customtabs.CustomTabsCallback;
import androidx.browser.customtabs.CustomTabsClient;
import androidx.browser.customtabs.CustomTabsService;
import androidx.browser.customtabs.CustomTabsServiceConnection;
import androidx.browser.customtabs.CustomTabsSession;
import androidx.browser.trusted.TrustedWebActivityIntent;
import androidx.browser.trusted.TrustedWebActivityIntentBuilder;

public final class ChadActivity extends Activity {
    private static final String CHROME = "com.android.chrome";
    private static final int RELATION = CustomTabsService.RELATION_HANDLE_ALL_URLS;

    private final Handler handler = new Handler(Looper.getMainLooper());
    private final Runnable timeout = () -> fail(R.string.chad_verification_failed);
    private boolean bound;
    private boolean completed;
    private Uri origin;
    private CustomTabsSession session;

    private final CustomTabsServiceConnection connection = new CustomTabsServiceConnection() {
        @Override
        public void onCustomTabsServiceConnected(ComponentName name, CustomTabsClient client) {
            if (completed) return;
            if (!client.warmup(0)) {
                fail(R.string.chad_requires_chrome);
                return;
            }
            session = client.newSession(new CustomTabsCallback() {
                @Override
                public void onRelationshipValidationResult(int relation, Uri requestedOrigin,
                                                           boolean verified, Bundle extras) {
                    if (completed || isFinishing() || relation != RELATION
                        || !"https".equals(requestedOrigin.getScheme())
                        || !origin.getHost().equals(requestedOrigin.getHost())
                        || (requestedOrigin.getPort() != -1 && requestedOrigin.getPort() != 443)) {
                        return;
                    }
                    if (verified) launch();
                    else fail(R.string.chad_verification_failed);
                }
            });
            if (session == null) {
                fail(R.string.chad_requires_chrome);
            } else if (!session.validateRelationship(RELATION, origin, null)) {
                fail(R.string.chad_verification_failed);
            }
        }

        @Override
        public void onServiceDisconnected(ComponentName name) {
            fail(R.string.chad_requires_chrome);
        }
    };

    @Override
    protected void onCreate(Bundle state) {
        super.onCreate(state);
        origin = Uri.parse(getString(R.string.chad_origin));
        TextView status = new TextView(this);
        status.setGravity(Gravity.CENTER);
        status.setPadding(32, 32, 32, 32);
        status.setText(R.string.chad_checking);
        setContentView(status);
        bound = CustomTabsClient.bindCustomTabsService(this, CHROME, connection);
        if (bound) handler.postDelayed(timeout, 15000);
        else fail(R.string.chad_requires_chrome);
    }

    private void launch() {
        Uri startUrl = Uri.parse(getString(R.string.chad_start_url));
        TrustedWebActivityIntent launch = new TrustedWebActivityIntentBuilder(startUrl).build(session);
        launch.getIntent().setPackage(CHROME);
        try {
            launch.launchTrustedWebActivity(this);
            completed = true;
            finish();
        } catch (ActivityNotFoundException | SecurityException error) {
            fail(R.string.chad_requires_chrome);
        }
    }

    private void fail(int message) {
        if (completed || isFinishing() || isDestroyed()) return;
        completed = true;
        handler.removeCallbacks(timeout);
        new AlertDialog.Builder(this)
            .setTitle(R.string.chad_app_name)
            .setMessage(message)
            .setPositiveButton(android.R.string.ok, (dialog, which) -> finish())
            .setOnCancelListener(dialog -> finish())
            .show();
    }

    @Override
    protected void onDestroy() {
        handler.removeCallbacks(timeout);
        completed = true;
        if (bound) unbindService(connection);
        super.onDestroy();
    }
}
