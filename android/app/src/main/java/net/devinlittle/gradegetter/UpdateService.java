package net.devinlittle.gradegetter;

import android.app.Service;
import android.appwidget.AppWidgetManager;
import android.content.ComponentName;
import android.content.Intent;
import android.os.Handler;
import android.os.IBinder;
import android.widget.RemoteViews;

import org.json.JSONArray;
import org.json.JSONObject;

import java.io.BufferedReader;
import java.io.InputStreamReader;
import java.io.OutputStream;
import java.net.HttpURLConnection;
import java.net.URL;
import java.util.Iterator;

public class UpdateService extends Service {

    private Handler handler;
    private Runnable runnable;

    @Override
    public void onCreate() {
        super.onCreate();
        handler = new Handler();
        startRepeatingTask();
    }

    private void startRepeatingTask() {
        runnable = new Runnable() {
            @Override
            public void run() {
                fetchDataAndUpdateWidget();
                handler.postDelayed(this, 5000); // 5 seconds
            }
        };
        handler.post(runnable);
    }

    private void fetchDataAndUpdateWidget() {
        new Thread(() -> {
            try {
                URL url = new URL("http://home.devinlittle.net:3000/grades");
                HttpURLConnection conn = (HttpURLConnection) url.openConnection();
                conn.setRequestMethod("GET");
                /* conn.setRequestProperty("Content-Type", "application/json");
                conn.setDoOutput(true);

                String jsonInput = "{\n" +
                        "  \"token\": \"eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJzdWIiOiI2Yzg1ZGY4ZS04MmM1LTQ2MDItYTVmNi01NDdjNjg4Y2Q3YWQiLCJ1c2VybmFtZSI6ImRldmluIiwiaWF0IjoxNzU3ODY1Njk4LCJleHAiOjE3NTg0NzA0OTh9.31LvkvFtMGm1uO6Dlq7z_1tQ7-E_w2vI9xb6bxUdd9c\"\n" +
                        "}"; 

                OutputStream os = conn.getOutputStream();
              //  os.write(jsonInput.getBytes());
                os.flush();
                os.close(); */

                int responseCode = conn.getResponseCode();

                if (responseCode == HttpURLConnection.HTTP_OK) {
                    BufferedReader in = new BufferedReader(new InputStreamReader(conn.getInputStream()));
                    StringBuilder response = new StringBuilder();
                    String inputLine;

                    while ((inputLine = in.readLine()) != null) {
                        response.append(inputLine);
                    }
                    in.close();

                    String jsonResponse = response.toString();
                    JSONObject json = new JSONObject(jsonResponse);

                    StringBuilder resultBuilder = new StringBuilder();
                    Iterator<String> keys = json.keys();

                    while (keys.hasNext()) {
                        String subject = keys.next();
                        JSONArray grades = json.getJSONArray(subject);

                        // Get the first non-null grade (assumed Q1)
                        for (int i = 0; i < grades.length(); i++) {
                            if (!grades.isNull(i)) {
                                double grade = grades.getDouble(i);
                                resultBuilder.append(subject)
                                        .append(": ")
                                        .append(grade)
                                        .append("\n");
                                break;
                            }
                        }
                    }

                    String result = resultBuilder.length() > 0 ? resultBuilder.toString().trim() : "No Q1 grades found";
                    updateWidgetText(result);

                } else {
                    updateWidgetText("API error: " + responseCode);
                }

            } catch (Exception e) {
                e.printStackTrace();
                updateWidgetText("Error: " + e.getMessage());
            }
        }).start();
    }

    private void updateWidgetText(String text) {
        AppWidgetManager manager = AppWidgetManager.getInstance(this);
        ComponentName widget = new ComponentName(this, MyWidgetProvider.class);
        RemoteViews views = new RemoteViews(getPackageName(), R.layout.widget_layout);
        views.setTextViewText(R.id.widget_text, text);
        manager.updateAppWidget(widget, views);
    }

    @Override
    public IBinder onBind(Intent intent) {
        return null;
    }

    @Override
    public void onDestroy() {
        handler.removeCallbacks(runnable);
        super.onDestroy();
    }
    @Override
    public int onStartCommand(Intent intent, int flags, int startId) {
        fetchDataAndUpdateWidget();  // Trigger immediate update on service start
        return START_STICKY;
    }

}
