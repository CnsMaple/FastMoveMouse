#include "ShowTipsWidget.h"

ShowTipsWidget::ShowTipsWidget(QWidget *parent) : QWidget(parent)
{
    setWindowFlags(Qt::FramelessWindowHint | Qt::WindowStaysOnTopHint);
    setAttribute(Qt::WA_TranslucentBackground);
    setAttribute(Qt::WA_TransparentForMouseEvents);
    // 最大化
    // setWindowState(Qt::WindowMaximized);
}

void ShowTipsWidget::setRects(QList<QRect> rects)
{
    this->rectsMap.clear();
    QString keyStr = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    int keyStrSize = keyStr.size();
    // keyStrSize的三次方
    if (rects.size() > qPow(keyStrSize, 3))
    {
        qDebug() << "Too many rects:" << rects.size();
        return;
    }
    else
    {
        if (rects.size() > 0 && rects.size() <= qPow(keyStrSize, 1))
        {
            int i = 0;
            for (auto rect : rects)
            {
                rectsMap.insert(keyStr[i], rect);
                i++;
            }
        }
        else if (rects.size() <= qPow(keyStrSize, 2))
        {
            int i = 0;
            for (auto rect : rects)
            {
                rectsMap.insert(QString(keyStr[i / keyStrSize]) + QString(keyStr[i % keyStrSize]),
                                rect);
                i++;
            }
        }
        else if (rects.size() <= qPow(keyStrSize, 3))
        {
            int i = 0;
            for (auto rect : rects)
            {
                rectsMap.insert(QString(keyStr[i / keyStrSize / keyStrSize])
                                    + QString(keyStr[i % keyStrSize])
                                    + QString(keyStr[i % keyStrSize]),
                                rect);
                i++;
            }
        }
        else
        {
            qDebug() << "Too many rects str:" << rects.size();
            return;
        }
    }
}

bool ShowTipsWidget::removeOneRect(QString keyStr)
{
    if (keyStr.size() != 1)
    {
        return false;
    }
    QMap<QString, QRect> newRectsMap;
    for (auto it = rectsMap.begin(); it != rectsMap.end(); it++)
    {
        if (it.key().at(0) == keyStr.at(0))
        {
            // 移除第一个字符
            newRectsMap.insert(it.key().mid(1), it.value());
        }
    }

    if (newRectsMap.size() == 0)
    {
        rectsMap.clear();
        update();
        return true;
    }
    else if (newRectsMap.size() == 1)
    {
        rectsMap.clear();
        // 移动鼠标
        QPoint point = newRectsMap.begin().value().center();
        QCursor::setPos(point.x() + x(), point.y() + y());
        update();
        return true;
    }
    else
    {
        rectsMap = newRectsMap;
        update();
    }
    qDebug() << "Remove One:" << keyStr;
    return false;
}

void ShowTipsWidget::paintEvent(QPaintEvent *event)
{

    QFont te_font = this->font();
    QFontMetrics fm(te_font);
    int te_height = fm.height();
    // 获取字体宽度

    // 绘画矩形
    QPainter painter(this);
    painter.setBrush(QBrush(QColor("#ffd55b")));
    painter.setPen(QPen(QColor("#000000"), 1));
    painter.setRenderHint(QPainter::Antialiasing);
    // QList<QRect> rects;
    for (auto it = rectsMap.begin(); it != rectsMap.end(); it++)
    {
        // painter.drawRect(rect);
        // 获取绘画文本的大小
        QString text = it.key();
        QRect rect = it.value();
        if (rect.width() == 0 || rect.height() == 0)
        {
            continue;
        }
        int te_width = fm.horizontalAdvance(text) + 5;
        int middle_x = rect.x() + rect.width() / 2;
        int middle_y = rect.y() + rect.height() / 2;
        QRect te_rect(middle_x - te_width / 2, middle_y - te_height / 2, te_width, te_height);

        // for (auto rect : rects)
        // {
        //     if (rect.x() > te_rect.x() && rect.x() < te_rect.x() + te_rect.width()
        //         && rect.y() > te_rect.y() && rect.y() < te_rect.y() + te_rect.height())
        //     {
        //         qDebug() << "Contains";
        //         continue;
        //     }
        // }

        // painter.drawRect(te_rect);
        QPainterPath path;
        path.addRoundedRect(te_rect, 4, 4);
        painter.drawPath(path);
        painter.drawText(te_rect, Qt::AlignHCenter | Qt::AlignVCenter, QString(text));
        // rects.push_back(te_rect);
        // rects.append(QRect(middle_x - te_width / 2, middle_y - te_height / 2, te_width,
        // te_height));
    }
    // painter.setBrush(Qt::NoBrush);
    // painter.drawRect(QRect(0, 0, width(), height()));
    // mouse_event(MOUSEEVENTF_LEFTDOWN, 0, 0, 0, 0);
    // mouse_event(MOUSEEVENTF_LEFTUP, 0, 0, 0, 0);
    // painter.drawRect(QRect(0, 0, width(), height()));
}
