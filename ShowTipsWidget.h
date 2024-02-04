#pragma once

#include <QPainterPath>
#include <QPainter>
#include <QWidget>
#include <QKeyEvent>
#include <QApplication>
#include <windows.h>

class ShowTipsWidget : public QWidget
{
        Q_OBJECT;

    public:
        ShowTipsWidget(QWidget *parent = nullptr);
        ~ShowTipsWidget() = default;

        void setRects(QList<QRect> rects);
        bool removeOneRect(QString keyStr);
        void paintEvent(QPaintEvent *event) override;

        // void mousePressEvent(QMouseEvent *event) override;
        // void keyPressEvent(QKeyEvent *event) override;

    private:
        // QList<QRect> rects;
        QMap<QString, QRect> rectsMap;
};
