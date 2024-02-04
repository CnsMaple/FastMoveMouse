#pragma once

#include "qobject.h"
#include <QApplication>
#include <QDebug>
#include <QRect>
#include <QScreen>
#include <UIAutomationClient.h>

class UiSCan : public QObject
{
        Q_OBJECT;

    public:
        UiSCan(HWND hwnd, QObject *parent = 0);
        ~UiSCan();

        QList<QRect> getAllLeafRect();
        // QList<QRect> getRectsForWindow(IUIAutomationElementArray *elementArray);
        QScreen *getCurrentScreen();
        QRect changeRectsDpi(QRect rects, double scaleIn = 1);

        QRect getRootRect()
        {
            return changeRectsDpi(rootRect);
        }

    private:
        IUIAutomation *automation = nullptr;
        HWND hwnd = nullptr;
        QScreen *screen = nullptr;
        double scale = 1;
        QRect rootRect;
};
