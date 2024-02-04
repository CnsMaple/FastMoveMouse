#include "UiScan.h"

UiSCan::UiSCan(HWND hwnd, QObject *parent) : QObject(parent), hwnd(hwnd)
{
    if (hwnd == nullptr)
    {
        qDebug() << "hwnd is null";
        return;
    }
    getCurrentScreen();

    RECT tempRect;
    GetWindowRect(hwnd, &tempRect);
    qDebug() << "tempRect:" << tempRect.left << tempRect.top << tempRect.right << tempRect.bottom;
    this->rootRect = QRect(tempRect.left,
                           tempRect.top,
                           tempRect.right - tempRect.left,
                           tempRect.bottom - tempRect.top);

    CoInitialize(nullptr);
    CoCreateInstance(CLSID_CUIAutomation,
                     nullptr,
                     CLSCTX_INPROC_SERVER,
                     IID_IUIAutomation,
                     (void **)&automation);
}

UiSCan::~UiSCan()
{
    if (automation)
    {
        automation->Release();
    }
    CoUninitialize();
}

QList<QRect> UiSCan::getAllLeafRect()
{
    IUIAutomationElement *root = nullptr;
    automation->ElementFromHandle(hwnd, &root);

    // RECT rootRect_;
    // root->get_CurrentBoundingRectangle(&rootRect_);
    // qDebug() << "windowroot"
    //          << QRect(rootRect_.left,
    //                   rootRect_.top,
    //                   rootRect_.right - rootRect_.left,
    //                   rootRect_.bottom - rootRect_.top);

    QList<QRect> rects;
    if (!root)
    {
        return rects;
    }
    IUIAutomationCondition *condition = nullptr;

    automation->CreateTrueCondition(&condition);

    if (!condition)
    {
        return rects;
    }
    IUIAutomationElementArray *elementArray = nullptr;
    root->FindAll(TreeScope_Subtree, condition, &elementArray);

    int element_count = 0;
    elementArray->get_Length(&element_count);

    // hack: 这里有偏移问题，但是只有一点点，影响不大

    // if (IsZoomed(hwnd))
    // {
    //     qDebug() << "zoomed";
    //     rootRect = QRect(screen->geometry().x(),
    //                      screen->geometry().y(),
    //                      screen->availableSize().width() * scale,
    //                      screen->availableSize().height() * scale);
    // }

    // int offsetX = screen->geometry().x() - rootRect.x();
    // int offsetY = screen->geometry().y() - rootRect.y();
    // qDebug() << offsetX << offsetY;
    qDebug() << "root:" << rootRect;
    for (int i = element_count - 1; i >= 0; i--)
    {
        IUIAutomationElement *element = nullptr;
        elementArray->GetElement(i, &element);
        // 判断是否最后一个
        IUIAutomationElement *childElement = nullptr;

        element->FindFirst(TreeScope_Children, condition, &childElement);
        if (childElement == nullptr)
        {
            RECT rect;
            element->get_CurrentBoundingRectangle(&rect);

            rects.append(changeRectsDpi(
                QRect(rect.left - rootRect.left() - screen->geometry().x() * (scale - 1),
                      rect.top - rootRect.top() - screen->geometry().y() * (scale - 1),
                      rect.right - rect.left,
                      rect.bottom - rect.top)));
        }
    }

    // qDebug() << "rects:" << rects;

    return rects;
}

QScreen *UiSCan::getCurrentScreen()
{
    // 获取所有显示器
    QList<QScreen *> list = QGuiApplication::screens();
    for (auto screen : list)
    {
        // qDebug() << "screen:" << screen->name();
        qDebug() << "screen geo:" << screen->geometry();
        // qDebug() << "Device Pixel Ratio:" << screen->devicePixelRatio();
        // qDebug() << "screen:" << screen->availableGeometry();
        // if (screen->geometry().x() < (rect.x() + rect.width()) / 2
        //     && screen->geometry().y() < (rect.y() + rect.height()) / 2
        //     && screen->geometry().width() + screen->geometry().x() > (rect.x() + rect.width()) /
        //     2
        //     && screen->geometry().height() + screen->geometry().y()
        //            > (rect.y() + rect.height()) / 2)
        // {
        //     return screen;
        // }

        // 获取当前屏幕的物理位置和大小
        HMONITOR hMonitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        MONITORINFO monitorInfo = { sizeof(monitorInfo) };
        GetMonitorInfo(hMonitor, &monitorInfo);

        if (screen->geometry().x() == monitorInfo.rcMonitor.left
            && screen->geometry().y() == monitorInfo.rcMonitor.top)
        {
            qDebug() << "screen:" << screen->name();
            qDebug() << "monitorInfo:" << monitorInfo.rcMonitor.left << monitorInfo.rcMonitor.top
                     << monitorInfo.rcMonitor.right << monitorInfo.rcMonitor.bottom;
            this->screen = screen;
            // this->scale = screen->devicePixelRatio();
            this->scale = GetDpiForWindow(hwnd) / 96.0;
            qDebug() << "scale:" << scale;
            return screen;
        }
    }
    return nullptr;
}

QRect UiSCan::changeRectsDpi(QRect rects, double scaleIn)
{
    int tempX =
        (rects.x() - screen->geometry().x()) - ((rects.x() - screen->geometry().x()) / scale);
    int tempY =
        (rects.y() - screen->geometry().y()) - ((rects.y() - screen->geometry().y()) / scale);

    rects.setX(screen->geometry().x() + ((rects.x() - screen->geometry().x()) / scale));
    rects.setY(screen->geometry().y() + ((rects.y() - screen->geometry().y()) / scale));
    rects.setWidth((rects.width() - tempX) / scale);
    rects.setHeight((rects.height() - tempY) / scale);
    return rects;
}
